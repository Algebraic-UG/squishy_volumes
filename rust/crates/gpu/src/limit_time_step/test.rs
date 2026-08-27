// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use approx::assert_relative_eq;
use nalgebra::{Matrix1x3, Matrix3, stack};
use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};
use squishy_volumes_util::{SpecificParticleParameters, lambda, mu};

use crate::test_data::{
    test_inviscid_parameters, test_lame_parameters, test_position_gradients_random,
};

use super::*;

fn check(settings: Settings, input_data: InputData, expected: &[TimeStepLimits]) {
    let (gpu_limits_over_time, gpu_time_step) = run(
        settings,
        input_data.clone(),
        Parameters {
            current_step: input_data.limits_over_time.len() as u32 - 1,
        },
    );
    println!("gpu_limits_over_time: {gpu_limits_over_time:#?}");
    println!("gpu_time_step: {gpu_time_step:?}");

    assert_eq!(gpu_limits_over_time.len(), expected.len());
    for (cpu, gpu) in expected.iter().zip(gpu_limits_over_time) {
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

    let minimum_limits = expected.iter().cloned().reduce(|a, b| a.min(&b)).unwrap();
    let time_step = minimum_limits
        .time_step_by_deformation
        .min(minimum_limits.time_step_by_isolated)
        .min(
            minimum_limits
                .time_step_by_sound
                .min(minimum_limits.time_step_by_velocity),
        );
    assert_relative_eq!(
        time_step,
        gpu_time_step,
        epsilon = 0.000001,
        max_relative = 0.01
    );
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
        max_time_step: 1.,
        time_step_history_length: 10,
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
            particle_position_gradients: &[stack![
                Matrix3::identity();
                Matrix1x3::zeros()
            ]],
            particle_velocities: &[Vector4::zeros()],
            particle_velocity_gradients: &[stack![
                Matrix3::zeros();
                Matrix1x3::zeros()
            ]],
            limits_over_time: &[Default::default(), Default::default()],
        },
        &[
            TimeStepLimits {
                time_step_by_velocity: 3.4028235e38,
                time_step_by_deformation: 3.4028235e38,
                time_step_by_isolated: 3.4028235e38,
                time_step_by_sound: 3.4028235e38,
            },
            TimeStepLimits {
                time_step_by_velocity: 1.0,
                time_step_by_deformation: 20000000.0,
                time_step_by_isolated: 0.02309401,
                time_step_by_sound: 0.027255382,
            },
        ],
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
        max_time_step: 1.,
        time_step_history_length: 10,
    };

    let positions = many_positions();
    let n = positions.len();

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
            limits_over_time: &[Default::default()],
        },
        &[TimeStepLimits {
            time_step_by_velocity: 0.31904256,
            time_step_by_deformation: 0.20010836,
            time_step_by_isolated: 1.2476232e-5,
            time_step_by_sound: 6.694314e-6,
        }],
    );
}

fn run(
    settings: Settings,
    input_data: InputData<'_>,
    parameters: Parameters,
) -> (Vec<TimeStepLimits>, f32) {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), settings.clone(), input_data).unwrap();
    let limits_over_time = input.limits_over_time.clone();
    let limit_time_step = LimitTimeStep::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { time_step } = limit_time_step
        .record(&mut context, &mut (&mut encoder).into(), input, parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [limits_over_time, time_step]);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [limits_over_time, time_step] = downloads.try_into().unwrap();

    (
        limits_over_time.to_vec().unwrap(),
        time_step.to_vec().unwrap()[0],
    )
}
