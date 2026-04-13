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
    let (block_table, cell_ids_in, cell_ids_out) = run_prepare_grid(cell_size, positions, indices);
    println!("cell_ids_in: {cell_ids_in:?}");
    println!("cell_ids_out: {cell_ids_out:?}");

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
            assert!(block_and_index > 0);
            let block = block_and_index >> 29;
            let index = (block_and_index & index_mask) - 1;
            println!("maybe index: {index}, block: {block}");

            if *block_to_find == cell_ids_out[index as usize] + block_offset(block) {
                break;
            }

            println!("nope");

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
) -> (Vec<u32>, Vec<Vector4<i32>>, Vec<Vector4<i32>>) {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let workgroup_size = 64;
    let bit_count = 2;
    let dispatch_limit = u16::MAX as u32;

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
        dispatch_limit,
    };
    let color_cells = ColorCells2Settings {
        workgroup_size,
        dispatch_limit,
    };
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
            (&buffers.limits, "limits"),
            (&buffers.indirect, "indirect"),
            (&buffers.block_table, "block_table"),
            (&buffers.cell_ids_in, "cell_ids_in"),
            (&buffers.cell_ids_out, "cell_ids_out"),
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

    let [limits, indirect, block_table, cell_ids_in, cell_ids_out] = downloads.try_into().unwrap();

    println!("limits: {:?}", limits.to_vec::<u32>());
    println!("indirect: {:?}", indirect.to_vec::<u32>());
    println!("cell_ids_in: {:?}", cell_ids_in.to_vec::<Vector4<i32>>());
    println!("cell_ids_out: {:?}", cell_ids_out.to_vec::<Vector4<i32>>());

    (
        block_table.to_vec(),
        cell_ids_in.to_vec(),
        cell_ids_out.to_vec(),
    )
}
