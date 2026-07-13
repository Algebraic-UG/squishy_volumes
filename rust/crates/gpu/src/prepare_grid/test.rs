// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::iter::once;

use nalgebra::Vector3;

use super::*;

fn check(settings: Settings, positions_and_collider_bits: &[PositionAndColliderBits]) {
    let nodes = get_node_set(settings.grid_node_size, positions_and_collider_bits);

    let OutputData {
        indirect_nodes,
        hash_table,
        node_ids_and_collider_bits,
        hash_table_multi,
        multi_offsets,
        multi,
    } = run_prepare_grid(settings, positions_and_collider_bits);
    let num_nodes = indirect_nodes.len as usize;
    assert_eq!(nodes.len(), num_nodes);
    println!("num_nodes: {num_nodes}");

    assert_eq!(
        hash_table.iter().filter(|entry| **entry != 0).count(),
        num_nodes
    );

    assert_eq!(hash_table.len(), hash_table_multi.len());
    assert!(hash_table.len().is_power_of_two());
    let mask = hash_table.len() as u32 - 1;

    println!("checking hash table");
    for query in &nodes {
        let hash = node_id_and_collider_bits_to_murmur(query);
        let mut slot = hash & mask;

        let mut found = false;
        for _ in 0..hash_table.len() {
            let index = hash_table[slot as usize] as usize;
            if index == 0 {
                panic!("didn't find {query:?}");
            }
            let index = index - 1;
            if node_ids_and_collider_bits[index] == *query {
                found = true;
                break;
            }
            slot += 1;
            slot &= mask;
        }
        assert!(found);
    }

    println!("checking hash table multi");
    for query in &nodes {
        let hash = node_id_to_murmur(&query.node_id);
        let mut slot = hash & mask;

        let mut found = false;
        for _ in 0..hash_table_multi.len() {
            let index = hash_table_multi[slot as usize] as usize;
            if index == 0 {
                panic!("didn't find {query:?}");
            }
            let index = index - 1;
            if node_ids_and_collider_bits[index].node_id == query.node_id {
                found = true;
                break;
            }
            slot += 1;
            slot &= mask;
        }
        assert!(found);
    }

    println!("multi_offsets: {:?}", &multi_offsets[0..num_nodes]);
    println!("multi: {:?}", &multi[0..num_nodes]);
    for multi_range in multi_offsets
        .iter()
        .zip(
            multi_offsets
                .iter()
                .take(indirect_nodes.len as usize)
                .skip(1)
                .chain(once(&indirect_nodes.len)),
        )
        .map(|(s, e)| *s as usize..*e as usize)
        .filter(|range| !range.is_empty())
    {
        println!("range: {multi_range:?}");
        let node_id = node_ids_and_collider_bits[multi[multi_range.start] as usize].node_id;
        for multi_index in multi_range {
            let node_index = multi[multi_index] as usize;
            println!("{node_index}");
            assert_eq!(node_id, node_ids_and_collider_bits[node_index].node_id);
        }
    }
}

#[test]
fn test_single() {
    let positions = [PositionAndColliderBits {
        position: Vector3::zeros(),
        collider_bits: 0,
    }];

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            grid_node_size: 1.,
            table_tries: 50,
        },
        &positions,
    );
}

#[test]
fn test_simple() {
    let positions = [
        Vector3::new(-0.5, -0.5, -0.5),
        Vector3::new(-0.5, -0.5, 0.5),
        Vector3::new(-0.5, 0.5, -0.5),
        Vector3::new(-0.5, 0.5, 0.5),
        Vector3::new(0.5, -0.5, -0.5),
        Vector3::new(0.5, -0.5, 0.5),
        Vector3::new(0.5, 0.5, -0.5),
        Vector3::new(0.5, 0.5, 0.5),
    ];
    let positions_and_collider_bits = positions
        .into_iter()
        .map(|position| PositionAndColliderBits {
            position,
            collider_bits: 0,
        })
        .collect::<Vec<_>>();

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            grid_node_size: 1.1,
            table_tries: 50,
        },
        &positions_and_collider_bits,
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let n = 1000;
    let positions: Vec<f32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<f32>()
        .take(n * 3)
        .collect();
    let positions_and_collider_bits = positions
        .chunks_exact(3)
        .map(Vector3::from_column_slice)
        .zip(ChaCha8Rng::seed_from_u64(42).random_iter::<u32>())
        .map(|(position, collider_bits)| PositionAndColliderBits {
            position,
            collider_bits,
        })
        .collect::<Vec<_>>();

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            grid_node_size: 1337.,
            table_tries: 50,
        },
        &positions_and_collider_bits,
    );
}

#[test]
fn specific() {
    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            grid_node_size: 0.5,
            table_tries: 50,
        },
        &specific_positions_and_collider_bits(),
    );
}

fn run_prepare_grid(
    settings: Settings,
    positions_and_collider_bits: &[PositionAndColliderBits],
) -> OutputData {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        settings.clone(),
        positions_and_collider_bits,
    )
    .unwrap();
    let prepare_grid = PrepareGrid::new(&mut context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        indirect_nodes,
        hash_table,
        node_ids_and_collider_bits,
        hash_table_multi,
        multi_offsets,
        multi,
    } = prepare_grid
        .record(
            &mut context,
            &mut (&mut encoder).into(),
            input,
            Parameters {
                max_num_grid_nodes: (positions_and_collider_bits.len() as u32 * 27)
                    .try_into()
                    .unwrap(),
            },
        )
        .unwrap();

    let downloads = DownloadsToHost::new(
        &context,
        [
            indirect_nodes,
            hash_table,
            node_ids_and_collider_bits,
            hash_table_multi,
            multi_offsets,
            multi,
            context.status(),
        ],
    );
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [
        indirect_nodes,
        hash_table,
        node_ids_and_collider_bits,
        hash_table_multi,
        multi_offsets,
        multi,
        status,
    ] = downloads.try_into().unwrap();

    status.to_vec::<GpuStatus>()[0].to_result(&context).unwrap();

    OutputData {
        indirect_nodes: indirect_nodes.to_vec()[0],
        hash_table: hash_table.to_vec(),
        node_ids_and_collider_bits: node_ids_and_collider_bits.to_vec(),
        hash_table_multi: hash_table_multi.to_vec(),
        multi_offsets: multi_offsets.to_vec(),
        multi: multi.to_vec(),
    }
}
