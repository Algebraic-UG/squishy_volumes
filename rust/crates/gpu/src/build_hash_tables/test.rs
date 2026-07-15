// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use rustc_hash::FxHashMap;

use super::*;

fn check(settings: Settings, node_ids_and_collider_bits: &[NodeIdAndColliderBits]) {
    let OutputData {
        hash_table,
        hash_table_multi,
        multi_counts,
    } = run(settings, node_ids_and_collider_bits);

    println!("num_nodes: {}", node_ids_and_collider_bits.len());
    assert_eq!(multi_counts.len(), node_ids_and_collider_bits.len());

    assert_eq!(
        hash_table.iter().filter(|entry| **entry != 0).count(),
        node_ids_and_collider_bits.len()
    );

    println!("multi_counts: {multi_counts:?}");

    assert_eq!(hash_table.len(), hash_table_multi.len());
    assert!(hash_table.len().is_power_of_two());
    let mask = hash_table.len() as u32 - 1;

    println!("checking hash table");
    for query in node_ids_and_collider_bits {
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
    for query in node_ids_and_collider_bits {
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

    let mut counts: FxHashMap<Vector3<i32>, u32> = Default::default();
    for NodeIdAndColliderBits { node_id, .. } in node_ids_and_collider_bits {
        *counts.entry(*node_id).or_default() += 1;
    }
    let counts: Vec<u32> = node_ids_and_collider_bits
        .iter()
        .map(|node_id_and_collider_bits| counts[&node_id_and_collider_bits.node_id])
        .collect();

    assert_eq!(counts, multi_counts);
}

#[test]
fn test_single() {
    let positions = [PositionAndColliderBits {
        position: Vector3::zeros(),
        collider_bits: 0,
    }];
    let node_ids_and_collider_bits: Vec<_> = get_node_set(1., &positions).into_iter().collect();
    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            table_tries: 50,
        },
        &node_ids_and_collider_bits,
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
    let node_ids_and_collider_bits: Vec<_> = get_node_set(1.1, &positions_and_collider_bits)
        .into_iter()
        .collect();

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            table_tries: 50,
        },
        &node_ids_and_collider_bits,
    );
}

fn run(settings: Settings, node_id_and_collider_bits: &[NodeIdAndColliderBits]) -> OutputData {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        (u16::MAX as u32).try_into().unwrap(),
        settings.clone(),
        node_id_and_collider_bits,
    )
    .unwrap();
    let build_hash_tables = BuildHashTables::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        hash_table,
        hash_table_multi,
        multi_counts,
    } = build_hash_tables
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [hash_table, hash_table_multi, multi_counts]);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [hash_table, hash_table_multi, multi_counts] = downloads.try_into().unwrap();

    OutputData {
        hash_table: hash_table.to_vec().unwrap(),
        hash_table_multi: hash_table_multi.to_vec().unwrap(),
        multi_counts: multi_counts.to_vec().unwrap(),
    }
}
