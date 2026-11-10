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
    ParticleFlags, ParticleParameters, SINGULAR_VALUE_SEPARATION, T,
    double_partial_elastic_energy_inviscid_by_invariant_3,
    first_piola_stress_inviscid_svd_in_diagonal_space,
    first_piola_stress_neo_hookean_svd_in_diagonal_space,
    partial_elastic_energy_inviscid_by_invariant_3,
    second_derivative_inviscid_svd_in_diagonal_space,
    second_derivative_neo_hookean_svd_in_diagonal_space,
};
use nalgebra::{Matrix3, Vector3};

pub fn principal_stretches(position_gradient: &Matrix3<T>) -> Vector3<T> {
    position_gradient.svd(false, false).singular_values
}

// Effective time step restrictions for explicit MPM simulation 4.1 Sound Speed
pub fn limit_time_step_by_speed_of_sound(
    flags: &ParticleFlags,
    parameters: &ParticleParameters,
    principal_stretches: &Vector3<T>,
    grid_node_size: T,
) -> T {
    let s = principal_stretches;

    let j = s.product();

    // we need to use L'Hôpital in this case
    let xy_close = (s.x - s.y).abs() < SINGULAR_VALUE_SEPARATION;
    let yz_close = (s.y - s.z).abs() < SINGULAR_VALUE_SEPARATION;
    let zx_close = (s.z - s.x).abs() < SINGULAR_VALUE_SEPARATION;

    let first: Vector3<T>;
    let second: Matrix3<T>;

    // TODO: do something with viscosity?

    if flags.contains(ParticleFlags::IS_SOLID) {
        first = first_piola_stress_neo_hookean_svd_in_diagonal_space(
            parameters.mu(),
            parameters.lambda(),
            s,
        );
        second = second_derivative_neo_hookean_svd_in_diagonal_space(
            parameters.mu(),
            parameters.lambda(),
            s,
        );
    } else if flags.contains(ParticleFlags::IS_FLUID) {
        first = first_piola_stress_inviscid_svd_in_diagonal_space(
            parameters.bulk_modulus,
            parameters.exponent,
            s,
        );
        second = second_derivative_inviscid_svd_in_diagonal_space(
            parameters.bulk_modulus,
            parameters.exponent,
            s,
        );
    } else {
        unreachable!()
    }

    let kappa = [
        s.x * s.x * second.m11,
        s.y * s.y * second.m22,
        s.z * s.z * second.m33,
        s.x * s.y
            * if xy_close {
                (first.x + s.x * second.m11 - s.y * second.m21) / 2. / s.x
            } else {
                (s.x * first.x - s.y * first.y) / (s.x * s.x - s.y * s.y)
            },
        s.y * s.z
            * if yz_close {
                (first.y + s.y * second.m22 - s.z * second.m32) / 2. / s.y
            } else {
                (s.y * first.y - s.z * first.z) / (s.y * s.y - s.z * s.z)
            },
        s.z * s.x
            * if zx_close {
                (first.z + s.z * second.m33 - s.x * second.m13) / 2. / s.z
            } else {
                (s.z * first.z - s.x * first.x) / (s.z * s.z - s.x * s.x)
            },
    ]
    .into_iter()
    .max_by(T::total_cmp)
    .unwrap()
        / j;
    let has_no_stiffness = kappa <= 0.;
    if has_no_stiffness {
        return T::MAX;
    }
    let current_density = parameters.density / j;

    let c = (kappa / current_density).sqrt();

    grid_node_size / c
}

/// Limits the time step so that a particle alone in its grid neighbourhood stays stable.
///
/// Such a particle's velocity gradient changes per step by `-Δt · 4 / (ρ h²) · τ`,
/// where `4 / h²` is the inverse inertia of quadratic B-splines. In each isotropic mode
/// (shear and volume change), elasticity contributes a stiffness `a`, and the explicit
/// update stays stable while `a Δt² ≤ 4`.
///
/// At rest this is the single particle restriction of Bai & Schroeder 2022,
/// "Stability Analysis of Explicit MPM", Eq. (4), with `ξ = 4 / h²` and `r = 1`,
/// since the MLS force makes their `R` the identity. Solids take the stiffness at
/// their current stretch, which grows with the largest principal stretch but is
/// never taken below the rest stiffness.
pub fn limit_time_step_by_isolated_particles(
    flags: &ParticleFlags,
    parameters: &ParticleParameters,
    principal_stretches: &Vector3<T>,
    grid_node_size: T,
) -> T {
    let elastic = elastic_mode_stiffness(flags, parameters, principal_stretches, grid_node_size);
    limit_time_step_of_oscillator(elastic.shear).min(limit_time_step_of_oscillator(elastic.volume))
}

struct ModeCoefficients {
    shear: T,
    volume: T,
}

fn elastic_mode_stiffness(
    flags: &ParticleFlags,
    parameters: &ParticleParameters,
    principal_stretches: &Vector3<T>,
    grid_node_size: T,
) -> ModeCoefficients {
    if flags.contains(ParticleFlags::IS_SOLID) {
        let per_density = 4. / grid_node_size / grid_node_size / parameters.density;
        let stretched_mu =
            parameters.mu() * largest_squared_stretch_but_at_least_rest(principal_stretches);
        let lambda = parameters.lambda();
        ModeCoefficients {
            shear: 2. * stretched_mu * per_density,
            volume: (2. * stretched_mu + 3. * lambda) * per_density,
        }
    } else if flags.contains(ParticleFlags::IS_FLUID) {
        let time_step =
            limit_time_step_by_isolated_fluid(parameters, principal_stretches, grid_node_size);
        ModeCoefficients {
            shear: 0.,
            volume: 4. / time_step / time_step,
        }
    } else {
        unreachable!()
    }
}

fn largest_squared_stretch_but_at_least_rest(principal_stretches: &Vector3<T>) -> T {
    principal_stretches.abs().max().powi(2).max(1.)
}

// Effective time step restrictions for explicit MPM simulation,
// Technical document "Simple bounds"
fn limit_time_step_by_isolated_fluid(
    parameters: &ParticleParameters,
    principal_stretches: &Vector3<T>,
    grid_node_size: T,
) -> T {
    if parameters.bulk_modulus <= 0. {
        return T::MAX;
    }
    let j = principal_stretches.product();
    const K: T = 6.; // quadratic splines
    const D: T = 3.; // 3D
    let first = partial_elastic_energy_inviscid_by_invariant_3(
        parameters.bulk_modulus,
        parameters.exponent,
        j,
    );
    if (j - 1.).abs() > SINGULAR_VALUE_SEPARATION {
        return grid_node_size / j * (parameters.density * (j - 1.) / (K * first * D)).sqrt();
    }

    let second = double_partial_elastic_energy_inviscid_by_invariant_3(
        parameters.bulk_modulus,
        parameters.exponent,
        j,
    );

    grid_node_size * (parameters.density / (K * second * D)).sqrt()
}

fn limit_time_step_of_oscillator(stiffness: T) -> T {
    if stiffness <= 0. {
        return T::MAX;
    }
    2. / stiffness.sqrt()
}

// At least somewhat similar to
// Effective time step restrictions for explicit MPM simulation 4.2-4

pub fn limit_time_step_by_velocity(velocity: &Vector3<T>, grid_node_size: T) -> T {
    grid_node_size * 0.5 / velocity.norm().max(grid_node_size * 0.5)
}

pub fn limit_time_step_by_deformation(velocity_gradient: &Matrix3<T>) -> T {
    const DELTA: T = 0.2;
    velocity_gradient
        .iter()
        .map(|e| DELTA / e.abs().max(1e-8))
        .min_by(T::total_cmp)
        .unwrap()
}
