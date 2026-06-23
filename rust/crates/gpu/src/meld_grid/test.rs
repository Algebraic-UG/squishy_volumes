// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};
use rustc_hash::FxHashSet;
use smallvec::SmallVec;
use squishy_volumes_util::collider_bits;

use super::*;

fn check(settings: Settings, dispatch_limit: NonZeroU32, input_data: InputData) {
    let mut multi_map: FxHashMap<Vector3<i32>, SmallVec<[u32; 3]>> = Default::default();
    for (index, NodeIdAndColliderBits { node_id, .. }) in
        input_data.node_ids_and_collider_bits.iter().enumerate()
    {
        multi_map.entry(*node_id).or_default().push(index as u32);
    }

    let cpu_node_momentums: Vec<_> = input_data
        .node_ids_and_collider_bits
        .iter()
        .map(
            |NodeIdAndColliderBits {
                 node_id,
                 collider_bits,
             }| {
                multi_map[node_id]
                    .iter()
                    .filter(|node_index| {
                        collider_bits::compatible(
                            collider_bits,
                            &input_data.node_ids_and_collider_bits[**node_index as usize]
                                .collider_bits,
                        )
                    })
                    .map(|node_index| input_data.node_momentums_in[*node_index as usize])
                    .sum::<Vector4<f32>>()
            },
        )
        .collect();

    let gpu_node_momentums = run(settings, dispatch_limit, input_data.clone());

    for ((node_id_and_collider_bits, cpu), gpu) in input_data
        .node_ids_and_collider_bits
        .iter()
        .zip(cpu_node_momentums)
        .zip(gpu_node_momentums)
    {
        println!("{node_id_and_collider_bits:?}");
        check_iters(cpu.iter(), gpu.iter());
    }
}

#[test]
fn test_all_zero() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let settings = Settings { workgroup_size };

    let node_ids = [
        Vector3::new(0, 0, 0),
        Vector3::new(0, 0, 1),
        Vector3::new(0, 0, 2),
        Vector3::new(0, 1, 0),
        Vector3::new(0, 1, 1),
        Vector3::new(0, 1, 2),
        Vector3::new(0, 2, 0),
        Vector3::new(0, 2, 1),
        Vector3::new(0, 2, 2),
        Vector3::new(1, 0, 0),
        Vector3::new(1, 0, 1),
        Vector3::new(1, 0, 2),
        Vector3::new(1, 1, 0),
        Vector3::new(1, 1, 1),
        Vector3::new(1, 1, 2),
        Vector3::new(1, 2, 0),
        Vector3::new(1, 2, 1),
        Vector3::new(1, 2, 2),
        Vector3::new(2, 0, 0),
        Vector3::new(2, 0, 1),
        Vector3::new(2, 0, 2),
        Vector3::new(2, 1, 0),
        Vector3::new(2, 1, 1),
        Vector3::new(2, 1, 2),
        Vector3::new(2, 2, 0),
        Vector3::new(2, 2, 1),
        Vector3::new(2, 2, 2),
    ];
    let node_ids_and_collider_bits = node_ids
        .into_iter()
        .map(|node_id| NodeIdAndColliderBits {
            node_id,
            collider_bits: 0,
        })
        .collect::<Vec<_>>();
    let node_momentums_in = vec![Vector4::zeros(); node_ids_and_collider_bits.len()];

    check(
        settings,
        dispatch_limit,
        InputData {
            node_ids_and_collider_bits: &node_ids_and_collider_bits,
            node_momentums_in: &node_momentums_in,
        },
    );
}

#[test]
fn test_random() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let settings = Settings { workgroup_size };

    let mut rng = ChaCha8Rng::seed_from_u64(234);
    let n = 1000;

    let node_ids_and_collider_bits: FxHashSet<NodeIdAndColliderBits> = (0..n)
        .map(|_| NodeIdAndColliderBits {
            node_id: Vector3::new(
                rng.random_range(-10..10),
                rng.random_range(-10..10),
                rng.random_range(-10..10),
            ),
            collider_bits: rng.random(),
        })
        .collect();
    let node_ids_and_collider_bits: Vec<_> = node_ids_and_collider_bits.into_iter().collect();
    let node_momentums_in: Vec<_> = (0..n)
        .map(|_| {
            Vector4::new(
                rng.random_range(-1.0..1.),
                rng.random_range(-1.0..1.),
                rng.random_range(-1.0..1.),
                rng.random_range(0.1..10.),
            )
        })
        .collect();

    check(
        settings,
        dispatch_limit,
        InputData {
            node_ids_and_collider_bits: &node_ids_and_collider_bits,
            node_momentums_in: &node_momentums_in,
        },
    );
}

fn run(settings: Settings, dispatch_limit: NonZeroU32, input_data: InputData) -> Vec<Vector4<f32>> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        settings.clone(),
        dispatch_limit,
        input_data,
    );
    let meld_grid = MeldGrid::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { node_momentums_out } = meld_grid
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, node_momentums_out);
    download.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
