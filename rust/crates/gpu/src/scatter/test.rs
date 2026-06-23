// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix1x3, Matrix3, Vector3, stack};
use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};

use super::*;

fn check(settings: Settings, dispatch_limit: NonZeroU32, input_data: InputData) {
    let cpu_node_momentums = scatter_on_cpu(settings.grid_node_size, input_data.clone());
    let gpu_node_momentums = run(settings, dispatch_limit, input_data);

    assert_eq!(cpu_node_momentums.len(), gpu_node_momentums.len());

    for (cpu, gpu) in cpu_node_momentums.into_iter().zip(gpu_node_momentums) {
        check_iters(cpu.iter(), gpu.iter());
    }
}

#[test]
fn test_single_undeformed() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let grid_node_size = 1.;
    let settings = Settings {
        workgroup_size,
        grid_node_size,
    };

    let contributor_offsets = (0..27).collect::<Vec<_>>();
    let contributors = [0; 27];
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
    let particle_tmp = [Matrix4::new(
        0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 1., 1., 1., 1.,
    )];

    check(
        settings,
        dispatch_limit,
        InputData {
            contributor_offsets: &contributor_offsets,
            contributors: &contributors,
            node_ids_and_collider_bits: &node_ids_and_collider_bits,
            particle_tmp: &particle_tmp,
        },
    );
}

#[test]
fn test_many_random_props() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let grid_node_size = 0.5;
    let time_step = 0.001;
    let settings = Settings {
        workgroup_size,
        grid_node_size,
    };

    let positions = many_positions();
    let n = positions.len();
    let positions_and_collider_bits = positions
        .into_iter()
        .map(|position| PositionAndColliderBits {
            position: position.xyz(),
            collider_bits: 0,
        })
        .collect::<Vec<_>>();

    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let masses = (0..n)
        .map(|_| rng.random_range(0.01..0.05))
        .collect::<Vec<_>>();
    let initial_volumes = (0..n)
        .map(|_| rng.random_range(0.01..0.05))
        .collect::<Vec<_>>();

    let particle_parameters = test_lame_parameters()
        .chain(test_lame_parameters())
        .cycle()
        .take(n)
        .map(Into::into)
        .collect::<Vec<_>>();
    #[allow(clippy::toplevel_ref_arg)]
    let position_gradients = test_position_gradients_random(n)
        .into_iter()
        .map(|m| stack![m; Matrix1x3::zeros()])
        .collect::<Vec<_>>();
    let velocities = (0..n)
        .map(|_| {
            Vector4::new(
                rng.random_range(-1.0..1.),
                rng.random_range(-1.0..1.),
                rng.random_range(-1.0..1.),
                0.,
            )
        })
        .collect::<Vec<_>>();
    #[allow(clippy::toplevel_ref_arg)]
    let velocity_gradients = (0..n)
        .map(|_| {
            stack![
                Matrix3::new(
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                );
                Matrix1x3::zeros()
            ]
        })
        .collect::<Vec<_>>();

    let mut node_ids_and_collider_bits = Vec::new();
    let mut contributor_offsets = Vec::new();
    let mut contributors = Vec::new();
    let mut offset = 0;
    for (node, mut node_contributors) in
        get_contributors(settings.grid_node_size, &positions_and_collider_bits).into_iter()
    {
        contributor_offsets.push(offset);
        node_ids_and_collider_bits.push(node);
        offset += node_contributors.len() as u32;
        contributors.append(&mut node_contributors);
    }

    let particle_tmp = prepare_tmp_on_cpu(
        grid_node_size,
        time_step,
        prepare_tmp::InputData {
            particle_masses: &masses,
            particle_initial_volumes: &initial_volumes,
            particle_parameters: &particle_parameters,
            particle_positions_and_collider_bits: &positions_and_collider_bits,
            particle_position_gradients: &position_gradients,
            particle_velocities: &velocities,
            particle_velocity_gradients: &velocity_gradients,
        },
    );

    check(
        settings,
        dispatch_limit,
        InputData {
            contributor_offsets: &contributor_offsets,
            contributors: &contributors,
            node_ids_and_collider_bits: &node_ids_and_collider_bits,
            particle_tmp: &particle_tmp,
        },
    );
}

fn run(settings: Settings, dispatch_limit: NonZeroU32, input_data: InputData) -> Vec<Vector4<f32>> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings, dispatch_limit, input_data);
    let scatter = Scatter::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { node_momentums } = scatter
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, node_momentums);
    download.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
