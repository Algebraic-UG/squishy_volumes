// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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

// Effective time step restrictions for explicit MPM simulation 4.1 Sound Speed
pub fn limit_time_step_by_speed_of_sound(
    flags: &ParticleFlags,
    parameters: &ParticleParameters,
    position_gradient: &Matrix3<T>,
    grid_node_size: T,
) -> T {
    let s = position_gradient.svd(false, false).singular_values;

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
            &s,
        );
        second = second_derivative_neo_hookean_svd_in_diagonal_space(
            parameters.mu(),
            parameters.lambda(),
            &s,
        );
    } else if flags.contains(ParticleFlags::IS_FLUID) {
        first = first_piola_stress_inviscid_svd_in_diagonal_space(
            parameters.bulk_modulus,
            parameters.exponent,
            &s,
        );
        second = second_derivative_inviscid_svd_in_diagonal_space(
            parameters.bulk_modulus,
            parameters.exponent,
            &s,
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
    let current_density = parameters.density / j;

    let c = (kappa / current_density).sqrt();

    grid_node_size / c
}

pub fn limit_time_step_by_isolated_particles(
    flags: &ParticleFlags,
    parameters: &ParticleParameters,
    position_gradient: &Matrix3<T>,
    grid_node_size: T,
) -> T {
    // TODO: do someting with viscosity?
    if flags.contains(ParticleFlags::IS_SOLID) {
        // Stability analysis of explicit MPM, Technical document 3.12
        let xi = 3. / grid_node_size / grid_node_size;
        const R: T = 1.; // APIC & CPIC
        const K: T = 1.; // CPIC
        const D: T = 3.; // 3D

        (parameters.density
            / (xi * (R - K / 2.) * (parameters.mu() + D / 2. * parameters.lambda())))
        .sqrt()
    } else if flags.contains(ParticleFlags::IS_FLUID) {
        // Effective time step restrictions for explicit MPM simulation,
        // Technical document "Simple bounds"
        let j = position_gradient.determinant();
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
    } else {
        unreachable!()
    }
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
