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

use nalgebra::Matrix4x3;
use squishy_volumes_util::{
    sand_plasticity,
    test_data::{
        SandPlasticityCase, assert_relative_norm_close, sand_expansion_cases,
        sand_frictionless_compression_cases, sand_nearly_pure_compression_cases,
        sand_pure_compression_cases, test_solid_parameters,
    },
};

use crate::test_data::test_position_gradients_random;

use super::*;

const RELATIVE_TOLERANCE: f32 = 1e-4;

fn check(cases: &[SandPlasticityCase]) {
    let particle_parameters: Vec<ParticleParameters> =
        cases.iter().map(|case| case.parameters).collect();
    let padded_position_gradients: Vec<Matrix4x3<f32>> = cases
        .iter()
        .map(|case| case.position_gradient.insert_row(3, 0.))
        .collect();

    let gpu_position_gradients = run(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        },
        &particle_parameters,
        &padded_position_gradients,
    );

    for (case, gpu) in cases.iter().zip(gpu_position_gradients) {
        let actual = gpu.fixed_view::<3, 3>(0, 0).into();
        assert_relative_norm_close(&case.expected, &actual, RELATIVE_TOLERANCE);
    }
}

#[test]
fn random_matches_cpu() {
    let cases: Vec<SandPlasticityCase> = test_solid_parameters()
        .cycle()
        .zip(test_position_gradients_random(1000))
        .map(|(parameters, position_gradient)| SandPlasticityCase {
            parameters,
            position_gradient,
            expected: sand_plasticity(
                parameters.mu(),
                parameters.lambda(),
                parameters.sand_alpha,
                &position_gradient,
            ),
        })
        .collect();
    check(&cases);
}

#[test]
fn pure_compression_is_unchanged() {
    check(&sand_pure_compression_cases());
}

#[test]
fn nearly_pure_compression_is_unchanged() {
    check(&sand_nearly_pure_compression_cases());
}

#[test]
fn frictionless_compression_loses_shear() {
    check(&sand_frictionless_compression_cases());
}

#[test]
fn expansion_becomes_rotation() {
    check(&sand_expansion_cases());
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
