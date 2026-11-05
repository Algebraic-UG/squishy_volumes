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

use nalgebra::{Matrix3, Rotation3, Vector3};

use crate::{ParticleParameters, T, test_lame_parameters};

use super::*;

const RELATIVE_TOLERANCE: T = 1e-4;

fn shear_stiff_parameters() -> impl Iterator<Item = ParticleParameters> {
    test_lame_parameters()
        .map(|[youngs_modulus, poissons_ratio]| ParticleParameters {
            youngs_modulus,
            poissons_ratio,
            ..Default::default()
        })
        .filter(|parameters| parameters.mu() > 0.)
}

fn rotations() -> impl Iterator<Item = Matrix3<T>> {
    [
        Rotation3::identity(),
        Rotation3::from_euler_angles(0.3, -1.1, 2.4),
        Rotation3::from_axis_angle(&Vector3::y_axis(), 1.),
        Rotation3::from_euler_angles(-2.9, 0.7, -0.2),
    ]
    .into_iter()
    .map(|rotation| *rotation.matrix())
}

fn compressions() -> impl Iterator<Item = T> {
    [0.5, 0.8, 0.95, 0.999].into_iter()
}

fn apply(parameters: &ParticleParameters, position_gradient: &Matrix3<T>) -> Matrix3<T> {
    sand_plasticity(
        parameters.mu(),
        parameters.lambda(),
        parameters.sand_alpha,
        position_gradient,
    )
}

fn assert_close(expected: &Matrix3<T>, actual: &Matrix3<T>) {
    let error = (expected - actual).norm();
    let tolerance = RELATIVE_TOLERANCE * expected.norm();
    assert!(
        error <= tolerance,
        "expected {expected} but got {actual}, error={error}, tolerance={tolerance}",
    );
}

#[test]
fn pure_compression_is_unchanged() {
    for parameters in shear_stiff_parameters() {
        for rotation in rotations() {
            for compression in compressions() {
                let position_gradient = rotation * compression;
                assert_close(&position_gradient, &apply(&parameters, &position_gradient));
            }
        }
    }
}

#[test]
fn pure_compression_with_mixed_rotations_is_unchanged() {
    for parameters in shear_stiff_parameters() {
        for left in rotations() {
            for right in rotations() {
                for compression in compressions() {
                    let position_gradient =
                        left * Matrix3::from_diagonal_element(compression) * right.transpose();
                    assert_close(&position_gradient, &apply(&parameters, &position_gradient));
                }
            }
        }
    }
}

fn nearly_pure_compression_strain(compression: T, shear: T) -> Vector3<T> {
    Vector3::new(
        (compression * (1. + shear)).ln(),
        compression.ln(),
        (compression * (1. - shear)).ln(),
    )
}

fn expected_plastic_multiplier(parameters: &ParticleParameters, strain: &Vector3<T>) -> T {
    let volumetric_strain = strain.sum();
    let deviatoric_strain = strain - Vector3::repeat(volumetric_strain / 3.);
    deviatoric_strain.norm()
        + (3. * parameters.lambda() + 2. * parameters.mu()) / (2. * parameters.mu())
            * volumetric_strain
            * parameters.sand_alpha
}

#[test]
fn nearly_pure_compression_is_unchanged() {
    let shear = 1e-3;
    for parameters in shear_stiff_parameters() {
        for left in rotations() {
            for right in rotations() {
                for compression in [0.5, 0.8, 0.95] {
                    let strain = nearly_pure_compression_strain(compression, shear);
                    assert!(expected_plastic_multiplier(&parameters, &strain) <= 0.);

                    let position_gradient =
                        left * Matrix3::from_diagonal(&strain.map(T::exp)) * right.transpose();
                    assert_close(&position_gradient, &apply(&parameters, &position_gradient));
                }
            }
        }
    }
}

#[test]
fn nearly_pure_compression_without_friction_loses_shear() {
    let shear = 1e-3;
    for parameters in shear_stiff_parameters() {
        let parameters = ParticleParameters {
            sand_alpha: 0.,
            ..parameters
        };
        for left in rotations() {
            for right in rotations() {
                for compression in [0.5, 0.8, 0.95] {
                    let strain = nearly_pure_compression_strain(compression, shear);
                    let position_gradient =
                        left * Matrix3::from_diagonal(&strain.map(T::exp)) * right.transpose();

                    let volume_preserving_compression = (strain.sum() / 3.).exp();
                    let expected = left * volume_preserving_compression * right.transpose();
                    assert_close(&expected, &apply(&parameters, &position_gradient));
                }
            }
        }
    }
}

#[test]
fn pure_expansion_becomes_rotation() {
    for parameters in shear_stiff_parameters() {
        for left in rotations() {
            for right in rotations() {
                let position_gradient =
                    left * Matrix3::from_diagonal_element(1.2) * right.transpose();
                let expected = left * right.transpose();
                assert_close(&expected, &apply(&parameters, &position_gradient));
            }
        }
    }
}
