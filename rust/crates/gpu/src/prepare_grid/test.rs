// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::HashSet;

use nalgebra::Vector3;

use super::*;

fn check(settings: Settings, positions_and_collider_bits: &[PositionAndColliderBits]) {
    let mut nodes: HashSet<NodeIdAndColliderBits> = Default::default();
    for PositionAndColliderBits {
        position,
        collider_bits,
    } in positions_and_collider_bits
    {
        let low_node = position_to_low_node(settings.grid_node_size, position);
        for x in 0..3 {
            for y in 0..3 {
                for z in 0..3 {
                    nodes.insert(NodeIdAndColliderBits {
                        node_id: low_node + Vector3::new(x, y, z),
                        collider_bits: *collider_bits,
                    });
                }
            }
        }
    }

    let (indirect_nodes, hash_table, node_ids_and_collider_bits) =
        run_prepare_grid(settings, positions_and_collider_bits);
    let num_nodes = indirect_nodes[0].len as usize;
    println!("num_nodes: {num_nodes}");

    assert_eq!(
        hash_table.iter().filter(|entry| **entry != 0).count(),
        num_nodes
    );

    assert!(hash_table.len().is_power_of_two());
    let mask = hash_table.len() as u32 - 1;

    for query @ NodeIdAndColliderBits { node_id, .. } in nodes {
        let hash = node_id_to_murmur(&node_id);
        let mut slot = hash & mask;

        let mut found = false;
        for _ in 0..hash_table.len() {
            let index = hash_table[slot as usize] as usize;
            if index == 0 {
                panic!("didn't find {query:?}");
            }
            let index = index - 1;
            if node_ids_and_collider_bits[index] == query {
                found = true;
                break;
            }
            slot += 1;
            slot &= mask;
        }
        assert!(found);
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
        },
        &positions_and_collider_bits,
    );
}

fn run_prepare_grid(
    settings: Settings,
    positions_and_collider_bits: &[PositionAndColliderBits],
) -> (Vec<Indirect>, Vec<u32>, Vec<NodeIdAndColliderBits>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        settings.clone(),
        positions_and_collider_bits,
    );
    let prepare_grid = PrepareGrid::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        indirect_nodes,
        hash_table,
        node_ids_and_collider_bits,
    } = prepare_grid
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(
        &context,
        [indirect_nodes, hash_table, node_ids_and_collider_bits],
    );
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [indirect_nodes, hash_table, node_ids_and_collider_bits] = downloads.try_into().unwrap();

    (
        indirect_nodes.to_vec(),
        hash_table.to_vec(),
        node_ids_and_collider_bits.to_vec(),
    )
}
