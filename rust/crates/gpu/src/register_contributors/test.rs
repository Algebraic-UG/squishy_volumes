// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;

use super::*;

fn check(settings: Settings, positions_and_collider_bits: &[PositionAndColliderBits]) {
    let (node_ids_and_collider_bits, cpu_contributor_offsets, cpu_contributors) =
        contributors_on_cpu(settings.grid_node_size, positions_and_collider_bits);

    let (gpu_contributor_offsets, gpu_contributors) = run(
        settings,
        &node_ids_and_collider_bits,
        positions_and_collider_bits,
    );

    assert_eq!(cpu_contributor_offsets, gpu_contributor_offsets, "offsets");
    assert_eq!(cpu_contributors, gpu_contributors, "actual contributors");
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

fn run(
    settings: Settings,
    node_ids_and_collider_bits: &[NodeIdAndColliderBits],
    positions_and_collider_bits: &[PositionAndColliderBits],
) -> (Vec<u32>, Vec<u32>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        settings.clone(),
        node_ids_and_collider_bits,
        positions_and_collider_bits,
    );
    let register_contributors = RegisterContributors::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        contributor_offsets,
        contributors,
    } = register_contributors
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [contributor_offsets, contributors]);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [contributor_offsets, contributors] = downloads.try_into().unwrap();

    (contributor_offsets.to_vec(), contributors.to_vec())
}
