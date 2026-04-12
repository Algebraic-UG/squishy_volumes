// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::HashSet;

use nalgebra::Vector4;

use super::*;

fn check(cell_size: f32, positions: &[Vector4<f32>], indices: &[u32]) {
    let (block_table, cell_ids) = run_prepare_grid(cell_size, positions, indices);

    let mut blocks: HashSet<Vector4<i32>> = Default::default();
    for position in positions {
        let cell = position.map(|c| (c / cell_size).floor() as i32);
        for x in 0..2 {
            for y in 0..2 {
                for z in 0..2 {
                    blocks.insert(cell + Vector4::new(x, y, z, 0));
                }
            }
        }
    }
    let blocks: Vec<_> = blocks.into_iter().collect();

    let table_size = (positions.len() as u32 * 8 * 2).next_power_of_two();
    let table_mask = table_size - 1;
    let index_mask = (1 << 29) - 1;

    for (block_to_find, hash) in blocks.iter().zip(cells_to_murmur_on_cpu(&blocks)) {
        println!("searching for block: {block_to_find:?}");
        let mut slot = hash & table_mask;
        loop {
            let block_and_index = block_table[slot as usize];
            println!("maybe: {block_and_index}");
            assert!(block_and_index > 0);
            let block = block_and_index >> 29;
            let index = (block_and_index & index_mask) - 1;

            if *block_to_find == cell_ids[index as usize] + block_offset(block) {
                break;
            }

            slot += 1;
            slot &= table_mask;
        }
    }
}

#[test]
fn test_single() {
    let positions = [Vector4::zeros()];

    let cell_size = 1.;

    check(cell_size, &positions, &[0]);
}

#[test]
fn test_simple() {
    let positions = [
        Vector4::new(-0.5, -0.5, -0.5, 0.),
        Vector4::new(-0.5, -0.5, 0.5, 0.),
        Vector4::new(-0.5, 0.5, -0.5, 0.),
        Vector4::new(-0.5, 0.5, 0.5, 0.),
        Vector4::new(0.5, -0.5, -0.5, 0.),
        Vector4::new(0.5, -0.5, 0.5, 0.),
        Vector4::new(0.5, 0.5, -0.5, 0.),
        Vector4::new(0.5, 0.5, 0.5, 0.),
    ];

    let cell_size = 1.;

    check(
        cell_size,
        &positions,
        &(0..positions.len() as u32).collect::<Vec<_>>(),
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let cell_size = 1337.;

    let positions: Vec<f32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<f32>()
        .take(1000 * 4)
        .collect();
    let positions: Vec<Vector4<f32>> = positions
        .chunks_exact(4)
        .map(Vector4::from_column_slice)
        .collect();

    let mut indices: Vec<_> = (0..positions.len() as u32).collect();
    shuffle(&mut indices, 43);

    check(cell_size, &positions, &indices);
}

fn run_prepare_grid(
    cell_size: f32,
    positions: &[Vector4<f32>],
    indices: &[u32],
) -> (Vec<u32>, Vec<Vector4<i32>>) {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let workgroup_size = 64;
    let bit_count = 2;

    let prefix_sum = PrefixSumSettings { workgroup_size };
    let sort_positions_into_cells = SortPositionsIntoCellsSettings {
        positions_to_keys: PositionsToKeysSettings {
            workgroup_size,
            cell_size,
        },
        radix_sort: RadixSortSettings {
            count_subkeys: CountSubkeysSettings {
                workgroup_size,
                bit_count,
            },
            prefix_sum,
            reorder: ReorderSettings {
                workgroup_size,
                bit_count,
            },
        },
    };
    let permute_positions = PermutePositionsSettings { workgroup_size };
    let find_cell_boundaries = FindCellBoundariesSettings {
        workgroup_size,
        cell_size,
    };
    let build_cells = BuildCellsSettings {
        workgroup_size,
        cell_size,
    };
    let offsets_to_indirect = OffsetsToIndirectSettings {
        workgroup_size,
        dispatch_limit: u16::MAX as u32,
    };
    let generate_indices = GenerateIndicesSettings { workgroup_size };
    let color_cells = ColorCellsSettings { workgroup_size };
    let build_hash_table_colors = BuildHashTableColorsSettings { workgroup_size };
    let allocate_blocks = AllocateBlocksSettings {
        workgroup_size,
        prefix_sum,
    };

    let prepare_grid = PrepareGrid::new(
        &context,
        PrepareGridSettings {
            sort_positions_into_cells,
            permute_positions,
            find_cell_boundaries,
            prefix_sum,
            build_cells,
            offsets_to_indirect,
            generate_indices,
            color_cells,
            build_hash_table_colors,
            allocate_blocks,
        },
    );

    let buffers =
        prepare_grid.create_buffers(&context, PrepareGridBufferInput { positions, indices });
    let downloads = DownloadsToHost::new(
        &context,
        [
            (&buffers.block_table, "block_table"),
            (&buffers.cell_ids_out, "cell_ids"),
        ],
    );

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    prepare_grid.compute_in_pass(&context, &mut compute_pass, (&buffers).into(), ());

    drop(compute_pass);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let [block_table, cell_ids] = downloads.try_into().unwrap();

    (block_table.to_vec(), cell_ids.to_vec())
}
