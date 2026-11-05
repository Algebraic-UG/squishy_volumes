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

use nalgebra::{Matrix1x3, Matrix3, Matrix4x3, Rotation3, Vector3, stack};
use squishy_volumes_util::sand_plasticity;

use crate::test_data::test_position_gradients_random;

use super::*;

fn check(
    particle_parameters: &[ParticleParameters],
    particle_position_gradients: &[Matrix4x3<f32>],
) {
    let gpu_particle_position_gradients = run(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        },
        particle_parameters,
        particle_position_gradients,
    );

    let cpu_particle_position_gradients = particle_parameters
        .iter()
        .zip(particle_position_gradients)
        .map(|(parameters, position_gradient)| {
            sand_plasticity(
                parameters.mu(),
                parameters.lambda(),
                parameters.sand_alpha,
                &position_gradient.fixed_view::<3, 3>(0, 0).into(),
            )
        });

    for (cpu, gpu) in cpu_particle_position_gradients
        .into_iter()
        .zip(gpu_particle_position_gradients)
    {
        println!("{} vs {}", cpu, gpu.fixed_view::<3, 3>(0, 0));

        check_iters_by_norm(&cpu, gpu.fixed_view::<3, 3>(0, 0));
    }
}

#[test]
fn random() {
    let n = 1000;
    check(
        &squishy_volumes_util::test_lame_parameters()
            .cycle()
            .take(n)
            .map(|[youngs_modulus, poissons_ratio]| ParticleParameters {
                youngs_modulus,
                poissons_ratio,
                ..Default::default()
            })
            .collect::<Vec<_>>(),
        #[allow(clippy::toplevel_ref_arg)]
        &test_position_gradients_random(1000)
            .into_iter()
            .map(|m| stack![m; Matrix1x3::zeros()])
            .collect::<Vec<_>>(),
    );
}

const RELATIVE_TOLERANCE: f32 = 1e-4;

fn shear_stiff_parameters() -> Vec<ParticleParameters> {
    squishy_volumes_util::test_lame_parameters()
        .map(|[youngs_modulus, poissons_ratio]| ParticleParameters {
            youngs_modulus,
            poissons_ratio,
            ..Default::default()
        })
        .filter(|parameters| parameters.mu() > 0.)
        .collect()
}

fn rotations() -> Vec<Matrix3<f32>> {
    [
        Rotation3::identity(),
        Rotation3::from_euler_angles(0.3, -1.1, 2.4),
        Rotation3::from_axis_angle(&Vector3::y_axis(), 1.),
        Rotation3::from_euler_angles(-2.9, 0.7, -0.2),
    ]
    .into_iter()
    .map(|rotation| *rotation.matrix())
    .collect()
}

fn nearly_pure_compression_strain(compression: f32, shear: f32) -> Vector3<f32> {
    Vector3::new(
        (compression * (1. + shear)).ln(),
        compression.ln(),
        (compression * (1. - shear)).ln(),
    )
}

fn expected_plastic_multiplier(parameters: &ParticleParameters, strain: &Vector3<f32>) -> f32 {
    let volumetric_strain = strain.sum();
    let deviatoric_strain = strain - Vector3::repeat(volumetric_strain / 3.);
    deviatoric_strain.norm()
        + (3. * parameters.lambda() + 2. * parameters.mu()) / (2. * parameters.mu())
            * volumetric_strain
            * parameters.sand_alpha
}

fn check_against_expected(cases: &[(Matrix3<f32>, Matrix3<f32>)]) {
    let parameters = shear_stiff_parameters();
    let (particle_parameters, (position_gradients, expected)): (Vec<_>, (Vec<_>, Vec<_>)) =
        parameters
            .iter()
            .flat_map(|parameters| {
                cases
                    .iter()
                    .map(move |(input, expected)| (*parameters, (*input, *expected)))
            })
            .unzip();
    let padded_position_gradients: Vec<Matrix4x3<f32>> = position_gradients
        .iter()
        .map(|m| m.insert_row(3, 0.))
        .collect();

    let gpu_position_gradients = run(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        },
        &particle_parameters,
        &padded_position_gradients,
    );

    for ((input, expected), gpu) in position_gradients
        .iter()
        .zip(&expected)
        .zip(gpu_position_gradients)
    {
        let actual: Matrix3<f32> = gpu.fixed_view::<3, 3>(0, 0).into();
        let error = (expected - actual).norm();
        let tolerance = RELATIVE_TOLERANCE * expected.norm();
        assert!(
            error <= tolerance,
            "input {input} expected {expected} but got {actual}, error={error}, tolerance={tolerance}",
        );
    }
}

#[test]
fn pure_compression_is_unchanged() {
    let cases: Vec<_> = rotations()
        .into_iter()
        .flat_map(|left| rotations().into_iter().map(move |right| (left, right)))
        .flat_map(|(left, right)| {
            [0.5, 0.8, 0.95, 0.999].map(|compression| {
                let position_gradient =
                    left * Matrix3::from_diagonal_element(compression) * right.transpose();
                (position_gradient, position_gradient)
            })
        })
        .collect();
    check_against_expected(&cases);
}

#[test]
fn nearly_pure_compression_is_unchanged() {
    let shear = 1e-3;
    let compressions = [0.5, 0.8, 0.95];
    for parameters in shear_stiff_parameters() {
        for compression in compressions {
            let strain = nearly_pure_compression_strain(compression, shear);
            assert!(expected_plastic_multiplier(&parameters, &strain) <= 0.);
        }
    }

    let cases: Vec<_> = rotations()
        .into_iter()
        .flat_map(|left| rotations().into_iter().map(move |right| (left, right)))
        .flat_map(|(left, right)| {
            compressions.map(|compression| {
                let strain = nearly_pure_compression_strain(compression, shear);
                let position_gradient =
                    left * Matrix3::from_diagonal(&strain.map(f32::exp)) * right.transpose();
                (position_gradient, position_gradient)
            })
        })
        .collect();
    check_against_expected(&cases);
}

fn run(
    settings: Settings,
    particle_parameters: &[ParticleParameters],
    particle_position_gradients: &[Matrix4x3<f32>],
) -> Vec<Matrix4x3<f32>> {
    let mut context = get_shared_context();

    let input = Input::new(
        context.device(),
        particle_parameters,
        particle_position_gradients,
    )
    .unwrap();
    let particle_position_gradients = input.particle_position_gradients.clone();

    let sand = Sand::new(&mut context, settings).unwrap();
    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output = sand
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, particle_position_gradients);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
