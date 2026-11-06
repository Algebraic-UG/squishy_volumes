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

use nalgebra::{Matrix3, SMatrix, SVD, SVector, Vector1, Vector3};

use crate::{
    Matrix9, T, Vector9, double_partial_elastic_energy_inviscid_by_invariant_3,
    double_partial_elastic_energy_neo_hookean_by_invariant_3,
    double_partial_invariant_3_by_position_gradient, double_partial_invariant_3_by_svd,
    elastic_energy_inviscid, elastic_energy_inviscid_by_invariant, elastic_energy_neo_hookean,
    elastic_energy_neo_hookean_by_invariants, elastic_energy_stable_neo_hookean,
    first_piola_stress_inviscid, first_piola_stress_inviscid_svd,
    first_piola_stress_inviscid_svd_in_diagonal_space, first_piola_stress_neo_hookean,
    first_piola_stress_neo_hookean_svd, first_piola_stress_neo_hookean_svd_in_diagonal_space,
    first_piola_stress_stable_neo_hookean, first_piola_stress_stable_neo_hookean_svd,
    hessian_inviscid, hessian_neo_hookean, hessian_neo_hookean_svd, invariant_2, invariant_3,
    partial_elastic_energy_inviscid_by_invariant_3,
    partial_elastic_energy_neo_hookean_by_invariant_3, partial_invariant_2_by_position_gradient,
    partial_invariant_2_by_svd, partial_invariant_3_by_position_gradient,
    partial_invariant_3_by_svd, second_derivative_inviscid_svd_in_diagonal_space,
    second_derivative_neo_hookean_svd_in_diagonal_space,
    test_data::{
        PrincipalDeformation, assert_norm_negligible, test_general_deformations, test_mu_lambda,
        test_rotated_deformations,
    },
    test_inviscid_parameters,
};

const STEP: T = 1e-6;
const RELATIVE_TOLERANCE: T = 1e-6;
const TEST_INVARIANTS_3: [T; 6] = [0.3, 0.7, 0.99, 1., 1.4, 2.5];

fn flatten(matrix: &Matrix3<T>) -> Vector9<T> {
    Vector9::from_column_slice(matrix.as_slice())
}

fn unflatten(vector: &Vector9<T>) -> Matrix3<T> {
    Matrix3::from_column_slice(vector.as_slice())
}

fn central_difference<const N: usize, const M: usize>(
    function: impl Fn(&SVector<T, N>) -> SVector<T, M>,
    at: &SVector<T, N>,
) -> SMatrix<T, M, N> {
    let columns: [SVector<T, M>; N] = std::array::from_fn(|i| {
        let mut offset = SVector::<T, N>::zeros();
        offset[i] = STEP;
        (function(&(at + offset)) - function(&(at - offset))) / (2. * STEP)
    });
    SMatrix::from_columns(&columns)
}

fn scalar_derivative(function: impl Fn(T) -> T, at: T) -> T {
    central_difference(
        |x: &Vector1<T>| Vector1::new(function(x.x)),
        &Vector1::new(at),
    )
    .x
}

fn gradient_by_position_gradient(
    function: impl Fn(&Matrix3<T>) -> T,
    at: &Matrix3<T>,
) -> Matrix3<T> {
    let row = central_difference(
        |x: &Vector9<T>| Vector1::new(function(&unflatten(x))),
        &flatten(at),
    );
    unflatten(&row.transpose())
}

fn jacobian_by_position_gradient(
    function: impl Fn(&Matrix3<T>) -> Matrix3<T>,
    at: &Matrix3<T>,
) -> Matrix9<T> {
    central_difference(
        |x: &Vector9<T>| flatten(&function(&unflatten(x))),
        &flatten(at),
    )
}

fn gradient_by_singular_values(function: impl Fn(&Vector3<T>) -> T, at: &Vector3<T>) -> Vector3<T> {
    central_difference(|s: &Vector3<T>| Vector1::new(function(s)), at).transpose()
}

fn jacobian_by_singular_values(
    function: impl Fn(&Vector3<T>) -> Vector3<T>,
    at: &Vector3<T>,
) -> Matrix3<T> {
    central_difference(function, at)
}

fn assert_matches<const R: usize, const C: usize>(
    expected: &SMatrix<T, R, C>,
    actual: &SMatrix<T, R, C>,
    model_scale: T,
) {
    let scale = expected.norm().max(model_scale);
    assert_norm_negligible(&(expected - actual), scale, RELATIVE_TOLERANCE);
}

fn assert_scalar_matches(expected: T, actual: T, model_scale: T) {
    assert_matches(&Vector1::new(expected), &Vector1::new(actual), model_scale);
}

fn deformations() -> impl Iterator<Item = PrincipalDeformation> {
    let uniform = test_rotated_deformations([1., 1.3].map(Vector3::repeat));
    test_general_deformations().into_iter().chain(uniform)
}

fn position_gradients() -> impl Iterator<Item = Matrix3<T>> {
    deformations().map(|deformation| deformation.position_gradient())
}

fn singular_values() -> impl Iterator<Item = Vector3<T>> {
    deformations().map(|deformation| deformation.stretches)
}

struct Decomposition {
    u: Matrix3<T>,
    s: Vector3<T>,
    v_t: Matrix3<T>,
}

fn decompose(position_gradient: &Matrix3<T>) -> Decomposition {
    let SVD {
        u,
        singular_values,
        v_t,
    } = position_gradient.svd(true, true);
    Decomposition {
        u: u.unwrap(),
        s: singular_values,
        v_t: v_t.unwrap(),
    }
}

fn diagonal_energy(energy: impl Fn(&Matrix3<T>) -> T) -> impl Fn(&Vector3<T>) -> T {
    move |s| energy(&Matrix3::from_diagonal(s))
}

#[test]
fn invariant_gradients() {
    for position_gradient in position_gradients() {
        assert_matches(
            &gradient_by_position_gradient(invariant_2, &position_gradient),
            &partial_invariant_2_by_position_gradient(&position_gradient),
            1.,
        );
        assert_matches(
            &gradient_by_position_gradient(invariant_3, &position_gradient),
            &partial_invariant_3_by_position_gradient(&position_gradient),
            1.,
        );
        assert_matches(
            &jacobian_by_position_gradient(
                partial_invariant_3_by_position_gradient,
                &position_gradient,
            ),
            &double_partial_invariant_3_by_position_gradient(&position_gradient),
            1.,
        );
    }
}

#[test]
fn invariant_gradients_by_svd() {
    for position_gradient in position_gradients() {
        let Decomposition { u, s, v_t } = decompose(&position_gradient);
        let recompose = |diagonal: Vector3<T>| u * Matrix3::from_diagonal(&diagonal) * v_t;
        assert_matches(
            &partial_invariant_2_by_position_gradient(&position_gradient),
            &recompose(partial_invariant_2_by_svd(&s)),
            1.,
        );
        assert_matches(
            &partial_invariant_3_by_position_gradient(&position_gradient),
            &recompose(partial_invariant_3_by_svd(&s)),
            1.,
        );
    }
    for s in singular_values() {
        assert_matches(
            &gradient_by_singular_values(|s| s.product(), &s),
            &partial_invariant_3_by_svd(&s),
            1.,
        );
        assert_matches(
            &jacobian_by_singular_values(partial_invariant_3_by_svd, &s),
            &double_partial_invariant_3_by_svd(&s),
            1.,
        );
    }
}

#[test]
fn neo_hookean_derivatives_by_invariant_3() {
    let invariant_2 = 3.5;
    for [mu, lambda] in test_mu_lambda() {
        for invariant_3 in TEST_INVARIANTS_3 {
            assert_scalar_matches(
                scalar_derivative(
                    |j| elastic_energy_neo_hookean_by_invariants(mu, lambda, invariant_2, j),
                    invariant_3,
                ),
                partial_elastic_energy_neo_hookean_by_invariant_3(mu, lambda, invariant_3),
                mu + lambda,
            );
            assert_scalar_matches(
                scalar_derivative(
                    |j| partial_elastic_energy_neo_hookean_by_invariant_3(mu, lambda, j),
                    invariant_3,
                ),
                double_partial_elastic_energy_neo_hookean_by_invariant_3(mu, lambda, invariant_3),
                mu + lambda,
            );
        }
    }
}

#[test]
fn neo_hookean_stress_is_energy_gradient() {
    for [mu, lambda] in test_mu_lambda() {
        for position_gradient in position_gradients() {
            assert_matches(
                &gradient_by_position_gradient(
                    |f| elastic_energy_neo_hookean(mu, lambda, f),
                    &position_gradient,
                ),
                &first_piola_stress_neo_hookean(mu, lambda, &position_gradient),
                mu + lambda,
            );
        }
    }
}

#[test]
fn neo_hookean_stress_by_svd() {
    for [mu, lambda] in test_mu_lambda() {
        for position_gradient in position_gradients() {
            let Decomposition { u, s, v_t } = decompose(&position_gradient);
            assert_matches(
                &first_piola_stress_neo_hookean(mu, lambda, &position_gradient),
                &first_piola_stress_neo_hookean_svd(mu, lambda, &u, &s, &v_t),
                mu + lambda,
            );
        }
    }
}

#[test]
fn neo_hookean_hessian_is_stress_jacobian() {
    for [mu, lambda] in test_mu_lambda() {
        for position_gradient in position_gradients() {
            assert_matches(
                &jacobian_by_position_gradient(
                    |f| first_piola_stress_neo_hookean(mu, lambda, f),
                    &position_gradient,
                ),
                &hessian_neo_hookean(mu, lambda, &position_gradient),
                mu + lambda,
            );
        }
    }
}

#[test]
fn neo_hookean_hessian_by_svd() {
    for [mu, lambda] in test_mu_lambda() {
        for position_gradient in position_gradients() {
            let Decomposition { u, s, v_t } = decompose(&position_gradient);
            assert_matches(
                &hessian_neo_hookean(mu, lambda, &position_gradient),
                &hessian_neo_hookean_svd(mu, lambda, &u, &s, &v_t),
                mu + lambda,
            );
        }
    }
}

#[test]
fn neo_hookean_derivatives_in_diagonal_space() {
    for [mu, lambda] in test_mu_lambda() {
        for s in singular_values() {
            assert_matches(
                &gradient_by_singular_values(
                    diagonal_energy(|f| elastic_energy_neo_hookean(mu, lambda, f)),
                    &s,
                ),
                &first_piola_stress_neo_hookean_svd_in_diagonal_space(mu, lambda, &s),
                mu + lambda,
            );
            assert_matches(
                &jacobian_by_singular_values(
                    |s| first_piola_stress_neo_hookean_svd_in_diagonal_space(mu, lambda, s),
                    &s,
                ),
                &second_derivative_neo_hookean_svd_in_diagonal_space(mu, lambda, &s),
                mu + lambda,
            );
        }
    }
}

#[test]
fn stable_neo_hookean_stress_is_energy_gradient() {
    for [mu, lambda] in test_mu_lambda() {
        for position_gradient in position_gradients() {
            assert_matches(
                &gradient_by_position_gradient(
                    |f| elastic_energy_stable_neo_hookean(mu, lambda, f),
                    &position_gradient,
                ),
                &first_piola_stress_stable_neo_hookean(mu, lambda, &position_gradient),
                mu + lambda,
            );
        }
    }
}

#[test]
fn stable_neo_hookean_stress_by_svd() {
    for [mu, lambda] in test_mu_lambda() {
        for position_gradient in position_gradients() {
            let Decomposition { u, s, v_t } = decompose(&position_gradient);
            assert_matches(
                &first_piola_stress_stable_neo_hookean(mu, lambda, &position_gradient),
                &first_piola_stress_stable_neo_hookean_svd(mu, lambda, &u, &s, &v_t),
                mu + lambda,
            );
        }
    }
}

#[test]
fn inviscid_derivatives_by_invariant_3() {
    for (bulk_modulus, exponent) in test_inviscid_parameters() {
        for invariant_3 in TEST_INVARIANTS_3 {
            assert_scalar_matches(
                scalar_derivative(
                    |j| elastic_energy_inviscid_by_invariant(bulk_modulus, exponent, j),
                    invariant_3,
                ),
                partial_elastic_energy_inviscid_by_invariant_3(bulk_modulus, exponent, invariant_3),
                bulk_modulus,
            );
            assert_scalar_matches(
                scalar_derivative(
                    |j| partial_elastic_energy_inviscid_by_invariant_3(bulk_modulus, exponent, j),
                    invariant_3,
                ),
                double_partial_elastic_energy_inviscid_by_invariant_3(
                    bulk_modulus,
                    exponent,
                    invariant_3,
                ),
                bulk_modulus,
            );
        }
    }
}

#[test]
fn inviscid_stress_is_energy_gradient() {
    for (bulk_modulus, exponent) in test_inviscid_parameters() {
        for position_gradient in position_gradients() {
            assert_matches(
                &gradient_by_position_gradient(
                    |f| elastic_energy_inviscid(bulk_modulus, exponent, f),
                    &position_gradient,
                ),
                &first_piola_stress_inviscid(bulk_modulus, exponent, &position_gradient),
                bulk_modulus,
            );
        }
    }
}

#[test]
fn inviscid_stress_by_svd() {
    for (bulk_modulus, exponent) in test_inviscid_parameters() {
        for position_gradient in position_gradients() {
            let Decomposition { u, s, v_t } = decompose(&position_gradient);
            assert_matches(
                &first_piola_stress_inviscid(bulk_modulus, exponent, &position_gradient),
                &first_piola_stress_inviscid_svd(bulk_modulus, exponent, &u, &s, &v_t),
                bulk_modulus,
            );
        }
    }
}

#[test]
fn inviscid_hessian_is_stress_jacobian() {
    for (bulk_modulus, exponent) in test_inviscid_parameters() {
        for position_gradient in position_gradients() {
            assert_matches(
                &jacobian_by_position_gradient(
                    |f| first_piola_stress_inviscid(bulk_modulus, exponent, f),
                    &position_gradient,
                ),
                &hessian_inviscid(bulk_modulus, exponent, &position_gradient),
                bulk_modulus,
            );
        }
    }
}

#[test]
fn inviscid_derivatives_in_diagonal_space() {
    for (bulk_modulus, exponent) in test_inviscid_parameters() {
        for s in singular_values() {
            assert_matches(
                &gradient_by_singular_values(
                    diagonal_energy(|f| elastic_energy_inviscid(bulk_modulus, exponent, f)),
                    &s,
                ),
                &first_piola_stress_inviscid_svd_in_diagonal_space(bulk_modulus, exponent, &s),
                bulk_modulus,
            );
            assert_matches(
                &jacobian_by_singular_values(
                    |s| {
                        first_piola_stress_inviscid_svd_in_diagonal_space(bulk_modulus, exponent, s)
                    },
                    &s,
                ),
                &second_derivative_inviscid_svd_in_diagonal_space(bulk_modulus, exponent, &s),
                bulk_modulus,
            );
        }
    }
}
