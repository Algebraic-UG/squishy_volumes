// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix1x3, Matrix3, Vector3, stack};
use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};
use squishy_volumes_util::{lambda, mu};

use crate::{
    particle_parameters::{Host, Solid},
    test_data::{test_inviscid_parameters, test_lame_parameters, test_position_gradients_random},
};

use super::*;

fn check(settings: Settings, input_data: InputData) {
    let cpu_particle_tmp = prepare_tmp_on_cpu(
        settings.grid_node_size,
        settings.time_step,
        input_data.clone(),
    );
    let gpu_particle_tmp = run(settings, input_data);

    for (cpu, gpu) in cpu_particle_tmp.into_iter().zip(gpu_particle_tmp) {
        check_iters_by_norm(cpu.iter(), gpu.iter());
    }
}

#[test]
fn test_single_undeformed() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let grid_node_size = 1.;
    let time_step = 0.001;
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        grid_node_size,
        time_step,
    };

    check(
        settings,
        InputData {
            particle_masses: &[1.],
            particle_initial_volumes: &[1.],
            particle_parameters: &[Host::Solid(Solid {
                mu: mu(1000., 0.3),
                lambda: lambda(1000., 0.3),
                viscosity: None,
                sand_alpha: None,
            })
            .into()],
            particle_positions_and_collider_bits: &[PositionAndColliderBits {
                position: Vector3::zeros(),
                collider_bits: 0,
            }],
            particle_position_gradients: &[stack![
                Matrix3::identity();
                Matrix1x3::zeros()
            ]],
            particle_velocities: &[Vector4::zeros()],
            particle_velocity_gradients: &[stack![
                Matrix3::zeros();
                Matrix1x3::zeros()
            ]],
        },
    );
}

#[test]
fn test_many_random_props() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let grid_node_size = 1.;
    let time_step = 0.001;
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        grid_node_size,
        time_step,
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
        .chain(test_inviscid_parameters())
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

    check(
        settings,
        InputData {
            particle_masses: &masses,
            particle_initial_volumes: &initial_volumes,
            particle_parameters: &particle_parameters,
            particle_positions_and_collider_bits: &positions_and_collider_bits,
            particle_position_gradients: &position_gradients,
            particle_velocities: &velocities,
            particle_velocity_gradients: &velocity_gradients,
        },
    );
}

fn run(settings: Settings, input_data: InputData<'_>) -> Vec<Matrix4<f32>> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), input_data);
    let prepare_tmp = PrepareTmp::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { particle_tmp } = prepare_tmp
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, particle_tmp);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let download = download.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
