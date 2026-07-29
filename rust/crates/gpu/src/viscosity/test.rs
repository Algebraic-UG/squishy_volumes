// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix3, Matrix4x3};
use squishy_volumes_file_frame::{SpecificParticleParameters, ViscosityParameters};
use squishy_volumes_util::cauchy_stress_general_viscosity;

use crate::test_data::test_velocity_gradients_random;

use super::*;

fn check(
    particle_parameters: &[ParticleParameters],
    particle_velocity_gradients: &[Matrix4x3<f32>],
) {
    let particle_flags: Vec<ParticleFlags> = particle_parameters.iter().map(Into::into).collect();

    let gpu_particle_stresses = run(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        },
        &particle_flags,
        &particle_parameters,
        particle_velocity_gradients,
    );

    let mut cpu_particle_velocity_gradients: Vec<Matrix3<f32>> = particle_velocity_gradients
        .iter()
        .map(|m| m.fixed_view::<3, 3>(0, 0).into())
        .collect();

    let cpu_particle_stresses = particle_parameters
        .iter()
        .zip(&mut cpu_particle_velocity_gradients)
        .map(|(parameters, velocity_gradient)| {
            if let Some(ViscosityParameters { dynamic, bulk }) = parameters.viscosity {
                cauchy_stress_general_viscosity(dynamic, bulk, velocity_gradient)
            } else {
                Matrix3::zeros()
            }
        })
        .collect::<Vec<_>>();

    for (cpu, gpu) in cpu_particle_stresses.into_iter().zip(gpu_particle_stresses) {
        println!("{} vs {}", cpu, gpu.fixed_view::<3, 3>(0, 0));

        check_iters_by_norm(&cpu, gpu.fixed_view::<3, 3>(0, 0));
    }
}

#[test]
fn random() {
    let n = 1000;
    check(
        &squishy_volumes_util::test_viscosity_parameters()
            .cycle()
            .take(n)
            .map(|[dynamic, bulk]| ParticleParameters {
                mass: 1.,
                initial_volume: 1.,
                viscosity: Some(ViscosityParameters { dynamic, bulk }),
                specific: SpecificParticleParameters::Solid {
                    mu: 0.,
                    lambda: 0.,
                    sand_alpha: None,
                },
            })
            .collect::<Vec<_>>(),
        &test_velocity_gradients_random(1000),
    );
}

fn run(
    settings: Settings,
    particle_flags: &[ParticleFlags],
    particle_parameters: &[ParticleParameters],
    particle_velocity_gradients: &[Matrix4x3<f32>],
) -> Vec<Matrix4x3<f32>> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        particle_flags,
        particle_parameters,
        particle_velocity_gradients,
    )
    .unwrap();

    let viscosity = Viscosity::new(&mut context, settings).unwrap();
    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { particle_stresses } = viscosity
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, particle_stresses);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
