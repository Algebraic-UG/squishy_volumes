// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::HashSet;

use super::*;

fn check(dispatch_limit: u32, workgroup_size: u32, cells: &[Vector4<i32>]) {
    let colorkeys = cells_to_colorkeys_on_cpu(&cells);

    let counts = (0..8)
        .map(|colorkey| colorkeys.iter().filter(|key| **key == colorkey).count() as u32)
        .collect::<Vec<_>>();
    // inclusive prefix sum here
    let limits: Vec<u32> = counts
        .iter()
        .scan(0, |prefix_sum, item| {
            *prefix_sum += item;
            Some(*prefix_sum)
        })
        .collect();
    let indirect = counts
        .iter()
        .flat_map(|count| find_x_y_z_simple(dispatch_limit, count.div_ceil(workgroup_size)))
        .collect::<Vec<_>>();

    let mut cells: Vec<(usize, &Vector4<i32>)> = cells.iter().enumerate().collect();
    cells.sort_by_key(|(index, _)| colorkeys[*index as usize]);
    let (_, cells): (Vec<_>, Vec<_>) = cells.into_iter().unzip();

    let (slots, owns) = run_build_hash_table_colors(
        workgroup_size,
        BuildHashTableColorsBufferInput {
            cells: &cells,
            limits: &limits,
            indirect: &indirect,
        },
    );

    println!("{slots:?}");
    println!("{owns:?}");
    let mut blocks: HashSet<Vector4<i32>> = Default::default();
    for cell in &cells {
        for x in 0..2 {
            for y in 0..2 {
                for z in 0..2 {
                    blocks.insert(cell + Vector4::new(x, y, z, 0));
                }
            }
        }
    }
    let blocks: Vec<_> = blocks.into_iter().collect();

    let table_size = (cells.len() as u32 * 8 * 2).next_power_of_two();
    let table_mask = table_size - 1;
    let index_mask = (1 << 29) - 1;

    for (block_to_find, hash) in blocks.iter().zip(cells_to_murmur_on_cpu(&blocks)) {
        println!("searching for block: {block_to_find:?}");
        let mut slot = hash & table_mask;
        loop {
            let block_and_index = slots[slot as usize];
            println!("maybe: {block_and_index}");
            assert!(block_and_index > 0);
            let block = block_and_index >> 29;
            let index = (block_and_index & index_mask) - 1;

            if *block_to_find == cells[index as usize] + block_offset(block) {
                assert!(owns[index as usize] & (1 << block) > 0);
                break;
            }

            slot += 1;
            slot &= table_mask;
        }
    }
}

#[test]
fn test_simple() {
    let cells = [
        Vector4::new(-5, -5, -5, 0),
        Vector4::new(-5, -5, 5, 0),
        Vector4::new(-5, 5, -5, 0),
        Vector4::new(-5, 5, 5, 0),
        Vector4::new(5, -5, -5, 0),
        Vector4::new(5, -5, 5, 0),
        Vector4::new(5, 5, -5, 0),
        Vector4::new(5, 5, 5, 0),
    ];

    let dispatch_limit = u16::MAX as u32;
    let workgroup_size = 64;

    check(dispatch_limit, workgroup_size, &cells);
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let cells: Vec<i32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<i32>()
        .take(1000 * 4)
        .collect();
    let cells: Vec<Vector4<i32>> = cells
        .chunks_exact(4)
        .map(Vector4::from_column_slice)
        .collect();

    let dispatch_limit = u16::MAX as u32;
    let workgroup_size = 64;

    check(dispatch_limit, workgroup_size, &cells);
}

fn run_build_hash_table_colors(
    workgroup_size: u32,
    buffer_input: BuildHashTableColorsBufferInput,
) -> (Vec<u32>, Vec<u32>) {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let build_hash_table_colors =
        BuildHashTableColors::new(&context, BuildHashTableColorsSettings { workgroup_size });
    let buffers = build_hash_table_colors.create_buffers(&context, buffer_input);

    let downloads = DownloadsToHost::new(
        &context,
        [(&buffers.slots, "slots"), (&buffers.owns, "owns")],
    );

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    build_hash_table_colors.compute_in_pass(&context, &mut compute_pass, (&buffers).into(), ());

    drop(compute_pass);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let [slots, owns] = downloads.try_into().unwrap();

    (slots.to_vec(), owns.to_vec())
}
