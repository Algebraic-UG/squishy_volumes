// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use approx::assert_relative_eq;
use itertools::izip;
use nalgebra::{Matrix1x3, Matrix3, stack};
use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};
use squishy_volumes_util::{
    SpecificParticleParameters, lambda, limit_time_step_by_deformation,
    limit_time_step_by_isolated_particles, limit_time_step_by_speed_of_sound,
    limit_time_step_by_velocity, mu,
};

use crate::test_data::{
    test_inviscid_parameters, test_lame_parameters, test_position_gradients_random,
};

use super::*;

fn check(
    settings @ Settings { grid_node_size, .. }: Settings,
    input_data @ InputData {
        particle_parameters,
        particle_position_gradients,
        particle_velocities,
        particle_velocity_gradients,
        ..
    }: InputData,
) {
    let cpu_time_step_limits: Vec<_> = izip!(
        particle_parameters,
        particle_position_gradients,
        particle_velocities,
        particle_velocity_gradients,
    )
    .map(
        |(parameters, position_gradient, velocity, velocity_gradient)| {
            let position_gradient: Matrix3<f32> = position_gradient.fixed_view::<3, 3>(0, 0).into();
            let velocity_gradient: Matrix3<f32> = velocity_gradient.fixed_view::<3, 3>(0, 0).into();
            TimeStepLimits {
                time_step_by_velocity: limit_time_step_by_velocity(&velocity.xyz(), grid_node_size),
                time_step_by_deformation: limit_time_step_by_deformation(&velocity_gradient),
                time_step_by_isolated: limit_time_step_by_isolated_particles(
                    parameters,
                    &position_gradient,
                    grid_node_size,
                ),
                time_step_by_sound: limit_time_step_by_speed_of_sound(
                    parameters,
                    &position_gradient,
                    grid_node_size,
                ),
            }
        },
    )
    .collect();

    let gpu_time_step_limits = run(settings, input_data.clone());

    println!("{gpu_time_step_limits:#?}");

    for (cpu, gpu) in cpu_time_step_limits.into_iter().zip(gpu_time_step_limits) {
        assert_relative_eq!(
            cpu.time_step_by_velocity,
            gpu.time_step_by_velocity,
            epsilon = 0.000001,
            max_relative = 0.01
        );
        assert_relative_eq!(
            cpu.time_step_by_deformation,
            gpu.time_step_by_deformation,
            epsilon = 0.000001,
            max_relative = 0.01
        );
        assert_relative_eq!(
            cpu.time_step_by_isolated,
            gpu.time_step_by_isolated,
            epsilon = 0.000001,
            max_relative = 0.01
        );
        assert_relative_eq!(
            cpu.time_step_by_sound,
            gpu.time_step_by_sound,
            epsilon = 0.000001,
            max_relative = 0.01
        );
    }
}

#[test]
fn test_single_undeformed() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let grid_node_size = 1.;
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        grid_node_size,
    };

    check(
        settings,
        InputData {
            particle_flags: &[ParticleFlags::IS_SOLID],
            particle_parameters: &[ParticleParameters {
                mass: 1.,
                initial_volume: 1.,
                viscosity: None,
                specific: SpecificParticleParameters::Solid {
                    mu: mu(1000., 0.3).unwrap(),
                    lambda: lambda(1000., 0.3).unwrap(),
                    sand_alpha: None,
                },
            }],
            #[allow(clippy::toplevel_ref_arg)]
            particle_position_gradients: &[stack![
                Matrix3::identity();
                Matrix1x3::zeros()
            ]],
            particle_velocities: &[Vector4::zeros()],
            #[allow(clippy::toplevel_ref_arg)]
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
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        grid_node_size,
    };

    let n = 1000;
    let mut rng = ChaCha8Rng::seed_from_u64(42);

    let particle_parameters = test_lame_parameters(&mut rng)
        .collect::<Vec<_>>()
        .into_iter()
        .chain(test_inviscid_parameters(&mut rng))
        .collect::<Vec<_>>()
        .into_iter()
        .cycle()
        .take(n)
        .collect::<Vec<_>>();
    let particle_flags = particle_parameters
        .iter()
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
            particle_flags: &particle_flags,
            particle_parameters: &particle_parameters,
            particle_position_gradients: &position_gradients,
            particle_velocities: &velocities,
            particle_velocity_gradients: &velocity_gradients,
        },
    );
}

fn run(settings: Settings, input_data: InputData<'_>) -> Vec<TimeStepLimits> {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), input_data).unwrap();
    let limit_time_step_per_particle =
        LimitTimeStepPerParticle::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { time_step_limits } = limit_time_step_per_particle
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, time_step_limits);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let download = download.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
