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

use crate::{
    ParticleParameters,
    test_data::{
        PrincipalDeformation, SandPlasticityCase, assert_relative_close,
        assert_relative_norm_close, sand_expansion_cases, sand_frictionless_compression_cases,
        sand_nearly_pure_compression_cases, sand_plastic_multiplier, sand_pure_compression_cases,
        test_general_deformations, test_nearly_pure_compressions, test_pure_compressions,
        test_pure_expansions, test_shear_stiff_parameters,
    },
};

use super::*;

const RELATIVE_TOLERANCE: T = 1e-4;

fn check(cases: &[SandPlasticityCase]) {
    for SandPlasticityCase {
        parameters,
        position_gradient,
        expected,
    } in cases
    {
        let actual = sand_plasticity(
            parameters.mu(),
            parameters.lambda(),
            parameters.sand_alpha,
            position_gradient,
        );
        assert_relative_norm_close(expected, &actual, RELATIVE_TOLERANCE);
    }
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

fn sand_parameters() -> impl Iterator<Item = ParticleParameters> {
    test_shear_stiff_parameters().flat_map(|parameters| {
        [0., 0.3, 1.].map(|sand_alpha| ParticleParameters {
            sand_alpha,
            ..parameters
        })
    })
}

fn deformations() -> Vec<PrincipalDeformation> {
    [
        test_general_deformations(),
        test_pure_compressions(),
        test_nearly_pure_compressions(),
        test_pure_expansions(),
    ]
    .concat()
}

fn project(parameters: &ParticleParameters, position_gradient: &Matrix3<T>) -> Matrix3<T> {
    sand_plasticity(
        parameters.mu(),
        parameters.lambda(),
        parameters.sand_alpha,
        position_gradient,
    )
}

#[test]
fn compression_outside_the_cone_lands_on_its_surface() {
    for parameters in sand_parameters() {
        for deformation in deformations() {
            let is_compressed = deformation.hencky_strain().sum() < 0.;
            let is_outside = sand_plastic_multiplier(&parameters, &deformation) > 0.;
            if !(is_compressed && is_outside) {
                continue;
            }
            let position_gradient = deformation.position_gradient();
            let projected = project(&parameters, &position_gradient);
            let projected_deformation = PrincipalDeformation::from_position_gradient(&projected);

            let strain_magnitude = deformation.hencky_strain().norm();
            let distance_to_surface = sand_plastic_multiplier(&parameters, &projected_deformation);
            assert!(
                distance_to_surface.abs() <= RELATIVE_TOLERANCE * strain_magnitude,
                "{deformation:?} ends at {distance_to_surface} from the cone surface"
            );
            assert_relative_close(
                position_gradient.determinant(),
                projected.determinant(),
                RELATIVE_TOLERANCE,
            );
            assert_relative_norm_close(
                &deformation.rotation(),
                &projected_deformation.rotation(),
                RELATIVE_TOLERANCE,
            );
        }
    }
}

#[test]
fn return_mapping_is_idempotent() {
    for parameters in sand_parameters() {
        for deformation in deformations() {
            let once = project(&parameters, &deformation.position_gradient());
            let twice = project(&parameters, &once);
            assert_relative_norm_close(&once, &twice, RELATIVE_TOLERANCE);
        }
    }
}
