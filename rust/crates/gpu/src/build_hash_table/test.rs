// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::HashSet;

use super::*;

fn check(workgroup_size: NonZeroU32, dispatch_limit: NonZeroU32, cells: &[Vector4<i32>]) {
    let (block_table, owns) = run_build_hash_table(workgroup_size, dispatch_limit, cells);

    println!("{block_table:?}");
    println!("{owns:?}");
    let mut blocks: HashSet<Vector4<i32>> = Default::default();
    for cell in cells {
        for x in 0..2 {
            for y in 0..2 {
                for z in 0..2 {
                    blocks.insert(cell + Vector4::new(x, y, z, 0));
                }
            }
        }
    }
    let blocks: Vec<_> = blocks.into_iter().collect();

    let table_size = block_table.len() as u32;
    assert!(table_size.is_power_of_two());
    let table_mask = table_size - 1;
    let index_mask = (1 << 29) - 1;

    for (block_to_find, hash) in blocks.iter().zip(cells_to_murmur_on_cpu(&blocks)) {
        println!("searching for block: {block_to_find:?}");
        let mut slot = hash & table_mask;
        let mut found = false;
        for _ in 0..block_table.len() {
            let block_and_index = block_table[slot as usize];
            println!("maybe: {block_and_index}");
            assert!(block_and_index > 0);
            let block = block_and_index >> 29;
            let index = (block_and_index & index_mask) - 1;

            if *block_to_find == cells[index as usize] + block_offset(block) {
                assert!(owns[index as usize] & (1 << block) > 0);
                found = true;
                break;
            }

            slot += 1;
            slot &= table_mask;
        }
        assert!(found);
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

    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let workgroup_size = 64.try_into().unwrap();

    check(workgroup_size, dispatch_limit, &cells);
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

    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let workgroup_size = 64.try_into().unwrap();

    check(workgroup_size, dispatch_limit, &cells);
}

fn run_build_hash_table(
    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
    cells: &[Vector4<i32>],
) -> (Vec<u32>, Vec<u32>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();
    let subgroup_size = context.subgroup_size();

    let input = Input::new(
        context.device(),
        workgroup_size,
        dispatch_limit,
        subgroup_size,
        cells,
    );
    let build_hash_table = BuildHashTable::new(&context, Settings { workgroup_size });

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { block_table, owns } = build_hash_table
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [block_table, owns]);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [block_table, owns] = downloads.try_into().unwrap();

    (block_table.to_vec(), owns.to_vec())
}
