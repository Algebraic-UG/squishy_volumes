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

use nalgebra::{Matrix3, Vector3};

use crate::{
    ParticleFlags, ParticleParameters, SINGULAR_VALUE_SEPARATION, T,
    first_piola_stress_neo_hookean, limit_time_step_by_deformation,
    limit_time_step_by_isolated_particles, limit_time_step_by_speed_of_sound,
    limit_time_step_by_velocity, principal_stretches,
    test_data::{
        assert_relative_close, test_general_deformations, test_rotations, test_solid_parameters,
    },
    test_inviscid_parameters,
};

const GRID_NODE_SIZE: T = 0.1;
const DENSITY: T = 4.;
const RELATIVE_TOLERANCE: T = 1e-9;

type Limit = fn(&ParticleFlags, &ParticleParameters, &Vector3<T>, T) -> T;

fn solids() -> impl Iterator<Item = ParticleParameters> {
    test_solid_parameters().map(|parameters| ParticleParameters {
        density: DENSITY,
        ..parameters
    })
}

fn stiff_solids() -> impl Iterator<Item = ParticleParameters> {
    solids().filter(|parameters| parameters.youngs_modulus > 0.)
}

fn fluid(bulk_modulus: T, exponent: i32) -> ParticleParameters {
    ParticleParameters {
        density: DENSITY,
        bulk_modulus,
        exponent,
        ..Default::default()
    }
}

fn fluids() -> impl Iterator<Item = ParticleParameters> {
    test_inviscid_parameters().map(|(bulk_modulus, exponent)| fluid(bulk_modulus, exponent))
}

fn solid_time_step(
    limit: Limit,
    parameters: &ParticleParameters,
    position_gradient: &Matrix3<T>,
) -> T {
    limit(
        &ParticleFlags::IS_SOLID,
        parameters,
        &principal_stretches(position_gradient),
        GRID_NODE_SIZE,
    )
}

fn fluid_time_step(
    limit: Limit,
    parameters: &ParticleParameters,
    position_gradient: &Matrix3<T>,
) -> T {
    limit(
        &ParticleFlags::IS_FLUID,
        parameters,
        &principal_stretches(position_gradient),
        GRID_NODE_SIZE,
    )
}

fn uniform(volume_ratio: T) -> Matrix3<T> {
    Matrix3::from_diagonal_element(volume_ratio.cbrt())
}

fn tait_sound_speed(parameters: &ParticleParameters, volume_ratio: T) -> T {
    let exponent = parameters.exponent;
    (exponent as T * parameters.bulk_modulus * volume_ratio.powi(1 - exponent) / parameters.density)
        .sqrt()
}

fn position_gradients() -> Vec<Matrix3<T>> {
    test_general_deformations()
        .into_iter()
        .map(|deformation| deformation.position_gradient())
        .collect()
}

#[test]
fn solid_sound_speed_at_rest_is_pressure_wave() {
    for parameters in stiff_solids() {
        let pressure_wave_speed =
            ((parameters.lambda() + 2. * parameters.mu()) / parameters.density).sqrt();
        assert_relative_close(
            GRID_NODE_SIZE / pressure_wave_speed,
            solid_time_step(
                limit_time_step_by_speed_of_sound,
                &parameters,
                &Matrix3::identity(),
            ),
            RELATIVE_TOLERANCE,
        );
    }
}

#[test]
fn fluid_sound_speed_follows_tait_equation() {
    for parameters in fluids() {
        for volume_ratio in [0.5, 0.9, 1., 1.1, 2.] {
            assert_relative_close(
                GRID_NODE_SIZE / tait_sound_speed(&parameters, volume_ratio),
                fluid_time_step(
                    limit_time_step_by_speed_of_sound,
                    &parameters,
                    &uniform(volume_ratio),
                ),
                RELATIVE_TOLERANCE,
            );
        }
    }
}

#[test]
fn fluid_sound_speed_ignores_shear() {
    for parameters in fluids() {
        for position_gradient in position_gradients() {
            let volume_ratio = position_gradient.determinant();
            assert_relative_close(
                GRID_NODE_SIZE / tait_sound_speed(&parameters, volume_ratio),
                fluid_time_step(
                    limit_time_step_by_speed_of_sound,
                    &parameters,
                    &position_gradient,
                ),
                RELATIVE_TOLERANCE,
            );
        }
    }
}

#[test]
fn sound_speed_is_frame_indifferent_and_isotropic() {
    for parameters in stiff_solids() {
        for position_gradient in position_gradients() {
            let reference = solid_time_step(
                limit_time_step_by_speed_of_sound,
                &parameters,
                &position_gradient,
            );
            for rotation in test_rotations() {
                for rotated in [rotation * position_gradient, position_gradient * rotation] {
                    assert_relative_close(
                        reference,
                        solid_time_step(limit_time_step_by_speed_of_sound, &parameters, &rotated),
                        RELATIVE_TOLERANCE,
                    );
                }
            }
        }
    }
}

#[test]
fn solid_sound_speed_is_continuous_at_repeated_singular_values() {
    let below = 0.9 * SINGULAR_VALUE_SEPARATION;
    let above = 1.1 * SINGULAR_VALUE_SEPARATION;
    for parameters in stiff_solids() {
        for (repeated, other) in [(1., 1.), (0.8, 1.3), (1.2, 0.7)] {
            for pair in [[0, 1], [1, 2], [2, 0]] {
                let with_separation = |separation: T| {
                    let mut singular_values = Vector3::repeat(other);
                    singular_values[pair[0]] = repeated;
                    singular_values[pair[1]] = repeated + separation;
                    solid_time_step(
                        limit_time_step_by_speed_of_sound,
                        &parameters,
                        &Matrix3::from_diagonal(&singular_values),
                    )
                };
                assert_relative_close(with_separation(above), with_separation(below), 1e-4);
            }
        }
    }
}

#[test]
fn stiffer_needs_smaller_steps() {
    let soft = ParticleParameters {
        youngs_modulus: 1e3,
        ..Default::default()
    };
    let stiff = ParticleParameters {
        youngs_modulus: 1e5,
        ..Default::default()
    };
    for limit in [
        limit_time_step_by_speed_of_sound as Limit,
        limit_time_step_by_isolated_particles,
    ] {
        for position_gradient in position_gradients() {
            assert!(
                solid_time_step(limit, &stiff, &position_gradient)
                    < solid_time_step(limit, &soft, &position_gradient)
            );
        }
    }
}

#[test]
fn isolated_fluid_is_continuous_at_rest() {
    let threshold = SINGULAR_VALUE_SEPARATION;
    for parameters in fluids() {
        let limit = |volume_ratio: T| {
            fluid_time_step(
                limit_time_step_by_isolated_particles,
                &parameters,
                &uniform(volume_ratio),
            )
        };
        let at_rest = limit(1.);
        for volume_ratio in [
            1. - 1.1 * threshold,
            1. - 0.9 * threshold,
            1. + 0.9 * threshold,
            1. + 1.1 * threshold,
        ] {
            assert_relative_close(at_rest, limit(volume_ratio), 1e-4);
        }
    }
}

#[test]
fn velocity_limits_travel_to_half_a_cell() {
    let direction = Vector3::new(1., -2., 0.5).normalize();
    for speed in [0., 1e-3, 0.05, 1., 100.] {
        let time_step = limit_time_step_by_velocity(&(direction * speed), GRID_NODE_SIZE);
        assert!(time_step > 0.);
        assert!(time_step * speed <= GRID_NODE_SIZE / 2. * (1. + RELATIVE_TOLERANCE));
    }
}

#[test]
fn deformation_limits_the_largest_entry_change() {
    let maximum_entry_change_per_step = 0.2;
    for position_gradient in position_gradients() {
        let velocity_gradient = position_gradient - Matrix3::identity();
        let time_step = limit_time_step_by_deformation(&velocity_gradient);
        let largest_rate = velocity_gradient.abs().max();
        assert_relative_close(
            maximum_entry_change_per_step,
            time_step * largest_rate,
            RELATIVE_TOLERANCE,
        );
    }
}

#[test]
fn deformation_limit_prevents_inversion_within_a_step() {
    let uniform_compression = Matrix3::repeat(-1.);
    let velocity_gradients = position_gradients()
        .into_iter()
        .map(|position_gradient| position_gradient - Matrix3::identity())
        .chain([-Matrix3::identity(), uniform_compression]);
    for velocity_gradient in velocity_gradients {
        for rate_scale in [1e-3, 1., 1e3] {
            let velocity_gradient = velocity_gradient * rate_scale;
            let time_step = limit_time_step_by_deformation(&velocity_gradient);
            let step_deformation = Matrix3::identity() + velocity_gradient * time_step;
            assert!(
                step_deformation.determinant() > 0.,
                "{velocity_gradient} inverts within {time_step}"
            );
        }
    }
}

#[test]
fn materials_without_stiffness_do_not_limit_the_time_step() {
    let soft_solid = ParticleParameters {
        density: DENSITY,
        youngs_modulus: 0.,
        ..Default::default()
    };
    let pressureless_fluid = fluid(0., 7);
    for limit in [
        limit_time_step_by_speed_of_sound as Limit,
        limit_time_step_by_isolated_particles,
    ] {
        for volume_ratio in [0.5, 0.9, 1., 1.1, 2.] {
            let position_gradient = uniform(volume_ratio);
            assert_eq!(
                T::MAX,
                solid_time_step(limit, &soft_solid, &position_gradient),
            );
            assert_eq!(
                T::MAX,
                fluid_time_step(limit, &pressureless_fluid, &position_gradient),
                "volume ratio {volume_ratio}"
            );
        }
    }
}

fn quadratic_spline_inverse_inertia() -> T {
    4. / GRID_NODE_SIZE / GRID_NODE_SIZE
}

fn viscous(base: ParticleParameters, dynamic: T, bulk: T) -> ParticleParameters {
    ParticleParameters {
        viscosity_dynamic: dynamic,
        viscosity_bulk: bulk,
        ..base
    }
}

fn viscous_solid_time_step(parameters: &ParticleParameters, position_gradient: &Matrix3<T>) -> T {
    limit_time_step_by_isolated_particles(
        &(ParticleFlags::IS_SOLID | ParticleFlags::USE_VISCOSITY),
        parameters,
        &principal_stretches(position_gradient),
        GRID_NODE_SIZE,
    )
}

#[test]
fn isolated_solid_at_rest_oscillates_in_its_volume_mode() {
    for parameters in stiff_solids() {
        let volume_stiffness = 2. * parameters.mu() + 3. * parameters.lambda();
        let angular_frequency =
            (quadratic_spline_inverse_inertia() * volume_stiffness / parameters.density).sqrt();
        assert_relative_close(
            2. / angular_frequency,
            solid_time_step(
                limit_time_step_by_isolated_particles,
                &parameters,
                &Matrix3::identity(),
            ),
            RELATIVE_TOLERANCE,
        );
    }
}

fn solids_across_poissons_ratios() -> impl Iterator<Item = ParticleParameters> {
    [-0.3, 0., 0.3, 0.45]
        .map(|poissons_ratio| ParticleParameters {
            density: DENSITY,
            youngs_modulus: 1e4,
            poissons_ratio,
            ..Default::default()
        })
        .into_iter()
}

fn stretched_and_compressed() -> Vec<Matrix3<T>> {
    let uniform = [0.6, 0.8, 1.2, 1.5, 2.].map(Matrix3::from_diagonal_element);
    let uniaxial =
        [0.6, 1.5, 2.].map(|stretch| Matrix3::from_diagonal(&Vector3::new(stretch, 1., 1.)));
    uniform
        .into_iter()
        .chain(uniaxial)
        .chain(position_gradients())
        .collect()
}

fn kirchhoff_stress_change(
    parameters: &ParticleParameters,
    left_stretch_squared: &Matrix3<T>,
    perturbation: &Matrix3<T>,
) -> Matrix3<T> {
    parameters.mu()
        * (perturbation * left_stretch_squared + left_stretch_squared * perturbation.transpose())
        + Matrix3::from_diagonal_element(parameters.lambda() * perturbation.trace())
}

fn assert_stress_change_matches_the_neo_hookean_stress(
    parameters: &ParticleParameters,
    position_gradient: &Matrix3<T>,
) {
    let kirchhoff_stress = |position_gradient: &Matrix3<T>| {
        first_piola_stress_neo_hookean(parameters.mu(), parameters.lambda(), position_gradient)
            * position_gradient.transpose()
    };
    let left_stretch_squared = position_gradient * position_gradient.transpose();
    let step = 1e-6;
    for index in 0..9 {
        let mut perturbation = Matrix3::zeros();
        perturbation[index] = 1.;
        let perturbed =
            |sign: T| (Matrix3::identity() + perturbation * (sign * step)) * position_gradient;
        let finite_difference =
            (kirchhoff_stress(&perturbed(1.)) - kirchhoff_stress(&perturbed(-1.))) / (2. * step);
        let expected = kirchhoff_stress_change(parameters, &left_stretch_squared, &perturbation);
        assert!(
            (finite_difference - expected).norm()
                <= 1e-6
                    * (parameters.mu() + parameters.lambda().abs())
                    * (1. + left_stretch_squared.norm()),
            "{position_gradient}"
        );
    }
}

fn largest_mode_stiffness(parameters: &ParticleParameters, position_gradient: &Matrix3<T>) -> T {
    let squared_stretches = principal_stretches(position_gradient).map(|stretch| stretch * stretch);
    let mu = parameters.mu();
    let shear_pairs =
        [(0, 1), (1, 2), (2, 0)].map(|(i, j)| mu * (squared_stretches[i] + squared_stretches[j]));
    let normal_block = Matrix3::from_diagonal(&(2. * mu * squared_stretches))
        + Matrix3::repeat(parameters.lambda());
    shear_pairs
        .into_iter()
        .chain(normal_block.symmetric_eigenvalues().iter().copied())
        .fold(0., T::max)
}

#[test]
fn isolated_solid_limit_covers_the_stiffness_at_its_current_stretch() {
    for parameters in solids_across_poissons_ratios() {
        for position_gradient in stretched_and_compressed() {
            assert_stress_change_matches_the_neo_hookean_stress(&parameters, &position_gradient);
            let angular_frequency = (quadratic_spline_inverse_inertia()
                * largest_mode_stiffness(&parameters, &position_gradient)
                / parameters.density)
                .sqrt();
            let stable_time_step = 2. / angular_frequency;
            let time_step = solid_time_step(
                limit_time_step_by_isolated_particles,
                &parameters,
                &position_gradient,
            );
            assert!(
                time_step <= stable_time_step * (1. + RELATIVE_TOLERANCE),
                "ν = {}, {position_gradient}: {time_step} exceeds {stable_time_step}",
                parameters.poissons_ratio,
            );
        }
    }
}

#[test]
fn isolated_viscous_particle_is_limited_by_its_decay_rate() {
    let without_stiffness = ParticleParameters {
        density: DENSITY,
        youngs_modulus: 0.,
        ..Default::default()
    };
    for [dynamic, bulk] in crate::test_viscosity_parameters().filter(|[d, b]| d + b > 0.) {
        for volume_ratio in [0.5, 1., 2.] {
            let current_density = DENSITY / volume_ratio;
            let decay_rate = quadratic_spline_inverse_inertia() * (2. * dynamic).max(3. * bulk)
                / current_density;
            assert_relative_close(
                2. / decay_rate,
                viscous_solid_time_step(
                    &viscous(without_stiffness, dynamic, bulk),
                    &uniform(volume_ratio),
                ),
                RELATIVE_TOLERANCE,
            );
        }
    }
}

#[test]
fn viscosity_shortens_the_time_step() {
    for parameters in stiff_solids() {
        let elastic = viscous_solid_time_step(&viscous(parameters, 0., 0.), &Matrix3::identity());
        assert_relative_close(
            solid_time_step(
                limit_time_step_by_isolated_particles,
                &parameters,
                &Matrix3::identity(),
            ),
            elastic,
            RELATIVE_TOLERANCE,
        );
        for [dynamic, bulk] in crate::test_viscosity_parameters().filter(|[d, b]| d + b > 0.) {
            let with_viscosity = viscous_solid_time_step(
                &viscous(parameters, dynamic * 1e3, bulk * 1e3),
                &Matrix3::identity(),
            );
            assert!(with_viscosity < elastic, "{dynamic} {bulk}");
        }
    }
}

#[test]
fn viscous_solid_sits_on_the_stability_boundary_of_its_weakest_mode() {
    for parameters in stiff_solids() {
        for [dynamic, bulk] in crate::test_viscosity_parameters().filter(|[d, b]| d + b > 0.) {
            let parameters = viscous(parameters, dynamic * 1e3, bulk * 1e3);
            let time_step = viscous_solid_time_step(&parameters, &Matrix3::identity());
            let per_density = quadratic_spline_inverse_inertia() / parameters.density;
            let modes = [
                (2. * parameters.mu(), 2. * parameters.viscosity_dynamic),
                (
                    2. * parameters.mu() + 3. * parameters.lambda(),
                    3. * parameters.viscosity_bulk,
                ),
            ];
            let largest = modes
                .into_iter()
                .map(|(stiffness, damping)| {
                    let stiffness = stiffness * per_density;
                    let damping = damping * per_density;
                    stiffness * time_step * time_step + 2. * damping * time_step
                })
                .fold(0., T::max);
            assert_relative_close(4., largest, 1e-9);
        }
    }
}
