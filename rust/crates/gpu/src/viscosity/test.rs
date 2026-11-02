// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use nalgebra::{Matrix3, Matrix4x3};
use squishy_volumes_util::cauchy_stress_general_viscosity;

use crate::test_data::test_velocity_gradients_random;

use super::*;

fn check(
    particle_parameters: &[ParticleParameters],
    particle_velocity_gradients: &[Matrix4x3<f32>],
) {
    let gpu_particle_stresses = run(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        },
        particle_parameters,
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
            cauchy_stress_general_viscosity(
                parameters.viscosity_dynamic,
                parameters.viscosity_bulk,
                velocity_gradient,
            )
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
            .map(|[viscosity_dynamic, viscosity_bulk]| ParticleParameters {
                viscosity_dynamic,
                viscosity_bulk,
                ..Default::default()
            })
            .collect::<Vec<_>>(),
        &test_velocity_gradients_random(1000),
    );
}

fn run(
    settings: Settings,
    particle_parameters: &[ParticleParameters],
    particle_velocity_gradients: &[Matrix4x3<f32>],
) -> Vec<Matrix4x3<f32>> {
    let mut context = get_shared_context();

    let input = Input::new(
        context.device(),
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
