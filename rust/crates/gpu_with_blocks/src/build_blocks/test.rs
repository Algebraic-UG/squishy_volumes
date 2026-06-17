// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::HashSet;

use super::*;

fn check(settings: Settings, cells: &[Vector4<i32>]) {
    let (indirect_blocks, block_ids, block_table) = run(settings, cells);

    println!("{indirect_blocks:?}");
    println!("{block_ids:?}");
    println!("{block_table:?}");
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
    let mut blocks: Vec<_> = blocks.into_iter().collect();
    blocks.sort_by(|a, b| {
        a.map(i32_to_u32_offset)
            .iter()
            .cmp(b.map(i32_to_u32_offset).iter())
    });

    assert!(block_ids.len() >= indirect_blocks[0].len as usize);
    {
        let mut block_ids: Vec<_> = block_ids
            .iter()
            .map(Vector4::xyz)
            .map(|v| v.push(0))
            .collect();
        block_ids.resize(indirect_blocks[0].len as usize, Vector4::zeros());
        block_ids.sort_by(|a, b| {
            a.map(i32_to_u32_offset)
                .iter()
                .cmp(b.map(i32_to_u32_offset).iter())
        });
        assert_eq!(blocks.len(), block_ids.len());
        for (cpu, gpu) in blocks.iter().zip(&block_ids) {
            assert_eq!(cpu.xyz(), gpu.xyz());
        }
    }

    let table_size = block_table.len() as u32;
    assert!(table_size.is_power_of_two());
    let table_mask = table_size - 1;

    for (block_to_find, hash) in blocks.iter().zip(cells_to_murmur_on_cpu(&blocks)) {
        println!("searching for block: {block_to_find:?}");
        let mut slot = hash & table_mask;
        let mut found = false;
        for _ in 0..block_table.len() {
            let index = block_table[slot as usize];
            assert!(index > 0);
            println!("maybe: {index}, aka. {}", block_ids[index as usize - 1]);
            if block_to_find.xyz() == block_ids[index as usize - 1].xyz() {
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
fn single() {
    let cells = [Vector4::zeros()];

    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let workgroup_size = 64.try_into().unwrap();

    check(
        Settings {
            workgroup_size,
            dispatch_limit,
        },
        &cells,
    );
}

#[test]
fn simple() {
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

    check(
        Settings {
            workgroup_size,
            dispatch_limit,
        },
        &cells,
    );
}

#[test]
fn random() {
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

    check(
        Settings {
            workgroup_size,
            dispatch_limit,
        },
        &cells,
    );
}

fn run(settings: Settings, cells: &[Vector4<i32>]) -> (Vec<Indirect>, Vec<Vector4<i32>>, Vec<u32>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings.clone(), cells);
    let build_hash_table = BuildBlocks::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        indirect_blocks,
        block_ids,
        block_table,
    } = build_hash_table
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [indirect_blocks, block_ids, block_table]);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [indirect_blocks, block_ids, block_table] = downloads.try_into().unwrap();

    (
        indirect_blocks.to_vec(),
        block_ids.to_vec(),
        block_table.to_vec(),
    )
}
