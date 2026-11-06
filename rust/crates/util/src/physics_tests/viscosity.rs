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

use nalgebra::{Matrix3, Vector1, Vector3};

use crate::{
    T, cauchy_stress_general_viscosity,
    test_data::{assert_close_at_scale, assert_norm_negligible, test_rotations},
    test_viscosity_parameters,
};

const RELATIVE_TOLERANCE: T = 1e-12;

fn simple_shear(shear_rate: T) -> Matrix3<T> {
    let mut velocity_gradient = Matrix3::zeros();
    velocity_gradient[(0, 1)] = shear_rate;
    velocity_gradient
}

fn velocity_gradients() -> Vec<Matrix3<T>> {
    let uniform_expansion = Vector3::repeat(1.);
    let uniform_compression = Vector3::repeat(-2.);
    let isochoric_stretching = Vector3::new(2., -1., -1.);
    let general_straining = Vector3::new(1.5, -0.4, 0.7);
    let principal_strain_rates = [
        uniform_expansion,
        uniform_compression,
        isochoric_stretching,
        general_straining,
    ];
    let straining_flows = principal_strain_rates.into_iter().flat_map(|rates| {
        test_rotations()
            .map(move |rotation| rotation * Matrix3::from_diagonal(&rates) * rotation.transpose())
    });

    let no_spin = Matrix3::zeros();
    let spin = Vector3::new(0.8, -1.3, 0.5).cross_matrix();
    straining_flows
        .flat_map(|straining| [no_spin, spin].map(|spin| straining + spin))
        .chain([simple_shear(3.)])
        .collect()
}

fn stress_scale(dynamic_viscosity: T, bulk_viscosity: T, velocity_gradient: &Matrix3<T>) -> T {
    (dynamic_viscosity + bulk_viscosity) * velocity_gradient.norm()
}

#[test]
fn stress_is_symmetric() {
    for [dynamic, bulk] in test_viscosity_parameters() {
        for velocity_gradient in velocity_gradients() {
            let stress = cauchy_stress_general_viscosity(dynamic, bulk, &velocity_gradient);
            assert_close_at_scale(
                &stress,
                &stress.transpose(),
                stress_scale(dynamic, bulk, &velocity_gradient),
                RELATIVE_TOLERANCE,
            );
        }
    }
}

#[test]
fn rigid_rotation_is_not_resisted() {
    for [dynamic, bulk] in test_viscosity_parameters() {
        for velocity_gradient in velocity_gradients() {
            let spin = (velocity_gradient - velocity_gradient.transpose()) / 2.;
            assert_norm_negligible(
                &cauchy_stress_general_viscosity(dynamic, bulk, &spin),
                stress_scale(dynamic, bulk, &velocity_gradient),
                RELATIVE_TOLERANCE,
            );
        }
    }
}

#[test]
fn only_bulk_viscosity_resists_expansion() {
    for [dynamic, bulk] in test_viscosity_parameters() {
        for velocity_gradient in velocity_gradients() {
            let mean_stress =
                cauchy_stress_general_viscosity(dynamic, bulk, &velocity_gradient).trace() / 3.;
            let divergence = velocity_gradient.trace();
            assert_close_at_scale(
                &Vector1::new(bulk * divergence),
                &Vector1::new(mean_stress),
                stress_scale(dynamic, bulk, &velocity_gradient),
                RELATIVE_TOLERANCE,
            );
        }
    }
}

#[test]
fn simple_shear_flow() {
    let shear_rate = 3.;
    let velocity_gradient = simple_shear(shear_rate);
    for [dynamic, bulk] in test_viscosity_parameters() {
        let mut expected = Matrix3::zeros();
        expected[(0, 1)] = dynamic * shear_rate;
        expected[(1, 0)] = dynamic * shear_rate;
        assert_close_at_scale(
            &expected,
            &cauchy_stress_general_viscosity(dynamic, bulk, &velocity_gradient),
            stress_scale(dynamic, bulk, &velocity_gradient),
            RELATIVE_TOLERANCE,
        );
    }
}

#[test]
fn viscous_dissipation_is_never_negative() {
    for [dynamic, bulk] in test_viscosity_parameters() {
        for velocity_gradient in velocity_gradients() {
            let stress = cauchy_stress_general_viscosity(dynamic, bulk, &velocity_gradient);
            let dissipation = stress.component_mul(&velocity_gradient).sum();
            let dissipation_scale =
                stress_scale(dynamic, bulk, &velocity_gradient) * velocity_gradient.norm();
            assert!(
                dissipation >= -RELATIVE_TOLERANCE * dissipation_scale,
                "{velocity_gradient}: {dissipation}"
            );
        }
    }
}
