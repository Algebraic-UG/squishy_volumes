// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;

use super::*;

fn check(node_ids_and_collider_bits: &[NodeIdAndColliderBits]) {
    let (hashes_node_ids, hashes_node_ids_and_collider_bits) = run(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
        },
        (u16::MAX as u32).try_into().unwrap(),
        node_ids_and_collider_bits,
    );

    println!("hashes_node_ids");
    assert_eq!(
        node_ids_and_collider_bits
            .iter()
            .map(|NodeIdAndColliderBits { node_id, .. }| node_id)
            .map(node_id_to_murmur)
            .collect::<Vec<_>>(),
        hashes_node_ids
    );
    println!("hashes_node_ids_and_collider_bits");
    assert_eq!(
        node_ids_and_collider_bits
            .iter()
            .map(node_id_and_collider_bits_to_murmur)
            .collect::<Vec<_>>(),
        hashes_node_ids_and_collider_bits
    );
}

#[test]
fn test_simple() {
    let node_ids = [
        Vector3::new(-5, -5, -5),
        Vector3::new(-5, -5, 5),
        Vector3::new(-5, 5, -5),
        Vector3::new(-5, 5, 5),
        Vector3::new(5, -5, -5),
        Vector3::new(5, -5, 5),
        Vector3::new(5, 5, -5),
        Vector3::new(5, 5, 5),
    ];

    let collider_bits = [
        0x0_0000, 0x1_0000, 0x2_0000, 0x3_0000, 0x4_0000, 0x5_0000, 0x6_0000, 0x7_0000,
    ];

    let node_ids_and_collider_bits: Vec<_> = node_ids
        .into_iter()
        .zip(collider_bits)
        .map(|(node_id, collider_bits)| NodeIdAndColliderBits {
            node_id,
            collider_bits,
        })
        .collect();
    check(&node_ids_and_collider_bits);
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let n = 1000;
    let node_ids: Vec<i32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<i32>()
        .take(n * 3)
        .collect();
    let node_ids: Vec<Vector3<i32>> = node_ids
        .chunks_exact(3)
        .map(Vector3::from_column_slice)
        .collect();
    let collider_bits = ChaCha8Rng::seed_from_u64(42).random_iter::<u32>();
    let node_ids_and_collider_bits: Vec<_> = node_ids
        .into_iter()
        .zip(collider_bits)
        .map(|(node_id, collider_bits)| NodeIdAndColliderBits {
            node_id,
            collider_bits,
        })
        .collect();
    check(&node_ids_and_collider_bits);
}

fn run(
    settings: Settings,
    dispatch_limit: NonZeroU32,
    node_ids_and_collider_bits: &[NodeIdAndColliderBits],
) -> (Vec<u32>, Vec<u32>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        settings.workgroup_size,
        dispatch_limit,
        node_ids_and_collider_bits,
    )
    .unwrap();

    let node_ids_to_murmur = NodeIdsToMurmur::new(&mut context, settings);
    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        hashes_node_ids,
        hashes_node_ids_and_collider_bits,
    } = node_ids_to_murmur
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(
        &context,
        [hashes_node_ids, hashes_node_ids_and_collider_bits],
    );
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [hashes_node_ids, hashes_node_ids_and_collider_bits] = downloads.try_into().unwrap();
    (
        hashes_node_ids.to_vec().unwrap(),
        hashes_node_ids_and_collider_bits.to_vec().unwrap(),
    )
}
