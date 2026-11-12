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
    SINGULAR_VALUE_SEPARATION, T, Vector9, elastic_energy_inviscid,
    elastic_energy_inviscid_by_invariant, elastic_energy_neo_hookean,
    elastic_energy_stable_neo_hookean, first_piola_stress_inviscid, first_piola_stress_neo_hookean,
    first_piola_stress_stable_neo_hookean, hessian_inviscid, hessian_neo_hookean,
    hessian_neo_hookean_svd, partial_elastic_energy_inviscid_by_invariant_3,
    test_data::{
        assert_close_at_scale, assert_norm_negligible, assert_relative_close,
        test_general_deformations, test_mu_lambda, test_pure_compressions, test_pure_expansions,
        test_rotations,
    },
    test_inviscid_parameters,
};

const RELATIVE_TOLERANCE: T = 1e-12;
const INVARIANCE_TOLERANCE: T = 1e-10;

type Energy = Box<dyn Fn(&Matrix3<T>) -> T>;
type Stress = Box<dyn Fn(&Matrix3<T>) -> Matrix3<T>>;

struct ElasticModel {
    energy: Energy,
    stress: Stress,
    stiffness: T,
}

fn elastic_models() -> Vec<ElasticModel> {
    let solids = test_mu_lambda().flat_map(|[mu, lambda]| {
        [
            ElasticModel {
                energy: Box::new(move |f| elastic_energy_neo_hookean(mu, lambda, f)),
                stress: Box::new(move |f| first_piola_stress_neo_hookean(mu, lambda, f)),
                stiffness: mu + lambda,
            },
            ElasticModel {
                energy: Box::new(move |f| elastic_energy_stable_neo_hookean(mu, lambda, f)),
                stress: Box::new(move |f| first_piola_stress_stable_neo_hookean(mu, lambda, f)),
                stiffness: mu + lambda,
            },
        ]
    });
    let fluids = test_inviscid_parameters().map(|(bulk_modulus, exponent)| ElasticModel {
        energy: Box::new(move |f| elastic_energy_inviscid(bulk_modulus, exponent, f)),
        stress: Box::new(move |f| first_piola_stress_inviscid(bulk_modulus, exponent, f)),
        stiffness: bulk_modulus,
    });
    solids.chain(fluids).collect()
}

fn position_gradients() -> Vec<Matrix3<T>> {
    test_general_deformations()
        .into_iter()
        .map(|deformation| deformation.position_gradient())
        .collect()
}

fn assert_scalar_close_at_scale(expected: T, actual: T, scale: T, relative_tolerance: T) {
    assert_close_at_scale(
        &Vector1::new(expected),
        &Vector1::new(actual),
        scale,
        relative_tolerance,
    );
}

#[test]
fn rotations_are_stress_free() {
    for model in elastic_models() {
        for rotation in test_rotations() {
            assert_norm_negligible(
                &Vector1::new((model.energy)(&rotation)),
                model.stiffness,
                RELATIVE_TOLERANCE,
            );
            assert_norm_negligible(
                &(model.stress)(&rotation),
                model.stiffness,
                RELATIVE_TOLERANCE,
            );
        }
    }
}

#[test]
fn frame_indifference() {
    for model in elastic_models() {
        for position_gradient in position_gradients() {
            for rotation in test_rotations() {
                let rotated = rotation * position_gradient;
                assert_scalar_close_at_scale(
                    (model.energy)(&position_gradient),
                    (model.energy)(&rotated),
                    model.stiffness,
                    INVARIANCE_TOLERANCE,
                );
                assert_close_at_scale(
                    &(rotation * (model.stress)(&position_gradient)),
                    &(model.stress)(&rotated),
                    model.stiffness,
                    INVARIANCE_TOLERANCE,
                );
            }
        }
    }
}

#[test]
fn isotropy() {
    for model in elastic_models() {
        for position_gradient in position_gradients() {
            for rotation in test_rotations() {
                let rotated = position_gradient * rotation;
                assert_scalar_close_at_scale(
                    (model.energy)(&position_gradient),
                    (model.energy)(&rotated),
                    model.stiffness,
                    INVARIANCE_TOLERANCE,
                );
                assert_close_at_scale(
                    &((model.stress)(&position_gradient) * rotation),
                    &(model.stress)(&rotated),
                    model.stiffness,
                    INVARIANCE_TOLERANCE,
                );
            }
        }
    }
}

#[test]
fn neo_hookean_energy_is_minimal_at_rest() {
    let deformations = test_general_deformations()
        .into_iter()
        .chain(test_pure_compressions())
        .chain(test_pure_expansions());
    for deformation in deformations {
        for [mu, lambda] in test_mu_lambda() {
            let energy = elastic_energy_neo_hookean(mu, lambda, &deformation.position_gradient());
            assert!(
                energy >= -RELATIVE_TOLERANCE * (mu + lambda),
                "{deformation:?}: {energy}"
            );
        }
    }
}

#[test]
fn inviscid_energy_grows_away_from_rest() {
    let compressions = (1..20).map(|i| i as T * 0.05);
    let expansions = (21..60).map(|i| i as T * 0.05);
    for (bulk_modulus, exponent) in test_inviscid_parameters() {
        let energy = |j: T| elastic_energy_inviscid_by_invariant(bulk_modulus, exponent, j);
        assert_norm_negligible(&Vector1::new(energy(1.)), bulk_modulus, RELATIVE_TOLERANCE);
        for j in compressions.clone() {
            assert!(energy(j) > energy(j + 0.05), "J = {j}");
        }
        for j in expansions.clone() {
            assert!(energy(j) > energy(j - 0.05), "J = {j}");
        }
    }
}

#[test]
fn inviscid_pressure_follows_tait_equation() {
    for (bulk_modulus, exponent) in test_inviscid_parameters() {
        for j in [0.5, 0.9, 0.99, 1.01, 1.1, 2.] {
            let pressure =
                -partial_elastic_energy_inviscid_by_invariant_3(bulk_modulus, exponent, j);
            let tait_pressure = bulk_modulus * (j.powi(-exponent) - 1.);
            assert_relative_close(tait_pressure, pressure, RELATIVE_TOLERANCE);
        }
    }
}

#[test]
fn neo_hookean_hessian_svd_near_repeated_singular_values() {
    let lhopital_tolerance = 10. * SINGULAR_VALUE_SEPARATION;
    let separations = [
        0.,
        1e-7,
        0.9 * SINGULAR_VALUE_SEPARATION,
        1.1 * SINGULAR_VALUE_SEPARATION,
        1e-4,
        1e-2,
    ];
    for [mu, lambda] in test_mu_lambda() {
        for separation in separations {
            for (repeated, other) in [(1., 1.), (0.8, 1.3), (1.2, 0.7)] {
                for (left, right) in test_rotations().zip(test_rotations().skip(1)) {
                    let singular_values = Vector3::new(repeated, repeated + separation, other);
                    let right_transposed = right.transpose();
                    let position_gradient =
                        left * Matrix3::from_diagonal(&singular_values) * right_transposed;
                    assert_close_at_scale(
                        &hessian_neo_hookean(mu, lambda, &position_gradient),
                        &hessian_neo_hookean_svd(
                            mu,
                            lambda,
                            &left,
                            &singular_values,
                            &right_transposed,
                        ),
                        mu + lambda,
                        lhopital_tolerance,
                    );
                }
            }
        }
    }
}

#[test]
fn energy_barrier_against_inversion() {
    let squashed = |s: T| Matrix3::from_diagonal(&Vector3::new(s, 1., 1.));
    let thicknesses: Vec<T> = (1..=8).map(|i| (10 as T).powi(-i)).collect();

    for [mu, lambda] in test_mu_lambda().filter(|[mu, _]| *mu > 0.) {
        for thickness in &thicknesses {
            let energy = elastic_energy_neo_hookean(mu, lambda, &squashed(*thickness));
            let lower_bound = -mu * thickness.ln() - mu / 2.;
            assert!(
                energy >= lower_bound,
                "{thickness}: {energy} < {lower_bound}"
            );
        }
    }

    for (bulk_modulus, exponent) in test_inviscid_parameters() {
        let energies: Vec<T> = thicknesses
            .iter()
            .map(|thickness| elastic_energy_inviscid(bulk_modulus, exponent, &squashed(*thickness)))
            .collect();
        assert!(energies.windows(2).all(|pair| pair[1] > pair[0]));
        for (thickness, energy) in thicknesses.iter().zip(&energies) {
            let lower_bound = bulk_modulus * (thickness.powi(1 - exponent) / (exponent - 1) as T)
                - bulk_modulus * exponent as T / (exponent - 1) as T;
            assert!(
                energy >= &lower_bound,
                "{thickness}: {energy} < {lower_bound}"
            );
        }
    }
}

#[test]
fn hessians_are_symmetric() {
    for position_gradient in position_gradients() {
        for [mu, lambda] in test_mu_lambda() {
            let hessian = hessian_neo_hookean(mu, lambda, &position_gradient);
            assert_close_at_scale(
                &hessian,
                &hessian.transpose(),
                mu + lambda,
                RELATIVE_TOLERANCE,
            );

            let svd = position_gradient.svd(true, true);
            let hessian = hessian_neo_hookean_svd(
                mu,
                lambda,
                &svd.u.unwrap(),
                &svd.singular_values,
                &svd.v_t.unwrap(),
            );
            assert_close_at_scale(
                &hessian,
                &hessian.transpose(),
                mu + lambda,
                INVARIANCE_TOLERANCE,
            );
        }
        for (bulk_modulus, exponent) in test_inviscid_parameters() {
            let hessian = hessian_inviscid(bulk_modulus, exponent, &position_gradient);
            assert_close_at_scale(
                &hessian,
                &hessian.transpose(),
                bulk_modulus,
                RELATIVE_TOLERANCE,
            );
        }
    }
}

#[test]
fn neo_hookean_resists_every_strain_near_rest() {
    let near_rest = |position_gradient: Matrix3<T>| {
        Matrix3::identity() + 0.05 * (position_gradient - Matrix3::identity())
    };
    let strains = position_gradients()
        .into_iter()
        .map(|position_gradient| position_gradient - Matrix3::identity())
        .map(|displacement_gradient| displacement_gradient + displacement_gradient.transpose());
    for [mu, lambda] in test_mu_lambda().filter(|[mu, _]| *mu > 0.) {
        for (position_gradient, strain) in position_gradients().into_iter().zip(strains.clone()) {
            let hessian = hessian_neo_hookean(mu, lambda, &near_rest(position_gradient));
            let strain = Vector9::from_column_slice(strain.as_slice());
            assert!(strain.dot(&(hessian * strain)) > 0.);
        }
    }
}
