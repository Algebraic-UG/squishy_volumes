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

use nalgebra::{Matrix3, SVD};

use crate::{
    Matrix9, T, first_piola_stress_inviscid, first_piola_stress_neo_hookean, hessian_inviscid,
    hessian_neo_hookean, hessian_neo_hookean_svd,
    test_data::{assert_close_at_scale, test_general_deformations, test_mu_lambda},
    test_inviscid_parameters,
};

const STEP: T = 1e-6;
const RELATIVE_TOLERANCE: T = 1e-6;
const EXACT_TOLERANCE: T = 1e-12;

fn linear_stress(mu: T, lambda: T, displacement_gradient: &Matrix3<T>) -> Matrix3<T> {
    mu * (displacement_gradient + displacement_gradient.transpose())
        + Matrix3::from_diagonal_element(lambda * displacement_gradient.trace())
}

fn kronecker_delta(a: usize, b: usize) -> T {
    if a == b { 1. } else { 0. }
}

fn linear_elasticity_tensor(mu: T, lambda: T) -> Matrix9<T> {
    let delta = kronecker_delta;
    Matrix9::from_fn(|row, column| {
        let (i, j) = (row % 3, row / 3);
        let (k, l) = (column % 3, column / 3);
        mu * (delta(i, k) * delta(j, l) + delta(i, l) * delta(j, k))
            + lambda * delta(i, j) * delta(k, l)
    })
}

fn stress_derivative_at_rest(
    stress: impl Fn(&Matrix3<T>) -> Matrix3<T>,
    direction: &Matrix3<T>,
) -> Matrix3<T> {
    let identity = Matrix3::identity();
    (stress(&(identity + direction * STEP)) - stress(&(identity - direction * STEP))) / (2. * STEP)
}

fn displacement_gradients() -> Vec<Matrix3<T>> {
    test_general_deformations()
        .into_iter()
        .map(|deformation| deformation.position_gradient() - Matrix3::identity())
        .collect()
}

#[test]
fn neo_hookean_small_strain() {
    for [mu, lambda] in test_mu_lambda() {
        for displacement_gradient in displacement_gradients() {
            assert_close_at_scale(
                &linear_stress(mu, lambda, &displacement_gradient),
                &stress_derivative_at_rest(
                    |f| first_piola_stress_neo_hookean(mu, lambda, f),
                    &displacement_gradient,
                ),
                mu + lambda,
                RELATIVE_TOLERANCE,
            );
        }
    }
}

#[test]
fn neo_hookean_hessian_at_rest_is_elasticity_tensor() {
    let identity = Matrix3::identity();
    let SVD {
        u,
        singular_values,
        v_t,
    } = identity.svd(true, true);
    for [mu, lambda] in test_mu_lambda() {
        let expected = linear_elasticity_tensor(mu, lambda);
        assert_close_at_scale(
            &expected,
            &hessian_neo_hookean(mu, lambda, &identity),
            mu + lambda,
            EXACT_TOLERANCE,
        );
        assert_close_at_scale(
            &expected,
            &hessian_neo_hookean_svd(mu, lambda, &u.unwrap(), &singular_values, &v_t.unwrap()),
            mu + lambda,
            EXACT_TOLERANCE,
        );
    }
}

#[test]
fn inviscid_small_strain_only_resists_volume_change() {
    for (bulk_modulus, exponent) in test_inviscid_parameters() {
        let linear_bulk_modulus = exponent as T * bulk_modulus;
        for displacement_gradient in displacement_gradients() {
            assert_close_at_scale(
                &linear_stress(0., linear_bulk_modulus, &displacement_gradient),
                &stress_derivative_at_rest(
                    |f| first_piola_stress_inviscid(bulk_modulus, exponent, f),
                    &displacement_gradient,
                ),
                bulk_modulus,
                RELATIVE_TOLERANCE,
            );
        }
        assert_close_at_scale(
            &linear_elasticity_tensor(0., linear_bulk_modulus),
            &hessian_inviscid(bulk_modulus, exponent, &Matrix3::identity()),
            bulk_modulus,
            EXACT_TOLERANCE,
        );
    }
}
