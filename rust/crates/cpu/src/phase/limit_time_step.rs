// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix3, Vector3};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use squishy_volumes_file_frame::{ParticleFlags, ParticleParameters, SpecificParticleParameters};

use squishy_volumes_util::{
    SINGULAR_VALUE_SEPARATION, double_partial_elastic_energy_inviscid_by_invariant_3,
    first_piola_stress_inviscid_svd_in_diagonal_space,
    first_piola_stress_neo_hookean_svd_in_diagonal_space,
    partial_elastic_energy_inviscid_by_invariant_3, profile,
    second_derivative_inviscid_svd_in_diagonal_space,
    second_derivative_neo_hookean_svd_in_diagonal_space,
};

use super::*;

impl CpuState {
    pub fn limit_time_step_before_force(&mut self, grid_node_size: f32) {
        profile!("limit_time_step_before_force");
        self.adaptive_time_step_state.time_step_by_sound =
            self.limit_time_step_by_speed_of_sound(grid_node_size);
        self.adaptive_time_step_state.time_step_by_isolated =
            self.limit_time_step_by_isolated_particles(grid_node_size);

        self.adaptive_time_step_state.push_current_limit();
    }

    // Effective time step restrictions for explicit MPM simulation 4.1 Sound Speed
    fn limit_time_step_by_speed_of_sound(&self, grid_node_size: f32) -> Option<f32> {
        profile!("limit_time_step_by_speed_of_sound");
        self.particles
            .parameters
            .par_iter()
            .zip(&self.particles.position_gradients)
            .zip(&self.particles.flags)
            .filter_map(|(e, flags)| (!flags.contains(ParticleFlags::TOMBSTONED)).then_some(e))
            .map(|(parameters, position_gradient)| {
                let s = position_gradient.svd(false, false).singular_values;

                let j = s.product();

                // we need to use L'Hôpital in this case
                let xy_close = (s.x - s.y).abs() < SINGULAR_VALUE_SEPARATION;
                let yz_close = (s.y - s.z).abs() < SINGULAR_VALUE_SEPARATION;
                let zx_close = (s.z - s.x).abs() < SINGULAR_VALUE_SEPARATION;

                let first: Vector3<f32>;
                let second: Matrix3<f32>;

                // TODO: do something with viscosity?

                match parameters.specific {
                    SpecificParticleParameters::Solid {
                        mu,
                        lambda,
                        sand_alpha: _,
                    } => {
                        first =
                            first_piola_stress_neo_hookean_svd_in_diagonal_space(mu, lambda, &s);
                        second =
                            second_derivative_neo_hookean_svd_in_diagonal_space(mu, lambda, &s);
                    }
                    SpecificParticleParameters::Fluid {
                        exponent,
                        bulk_modulus,
                    } => {
                        first = first_piola_stress_inviscid_svd_in_diagonal_space(
                            bulk_modulus,
                            exponent,
                            &s,
                        );
                        second = second_derivative_inviscid_svd_in_diagonal_space(
                            bulk_modulus,
                            exponent,
                            &s,
                        );
                    }
                }

                let kappa = [
                    s.x * s.x * second.m11,
                    s.y * s.y * second.m22,
                    s.z * s.z * second.m33,
                    s.y * s.y
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
                            (first.z + s.x * second.m33 - s.x * second.m13) / 2. / s.z
                        } else {
                            (s.z * first.z - s.x * first.x) / (s.z * s.z - s.x * s.x)
                        },
                ]
                .into_iter()
                .max_by(f32::total_cmp)
                .unwrap()
                    / j;
                let initial_density = parameters.mass / parameters.initial_volume;
                let current_density = initial_density / j;

                let c = (kappa / current_density).sqrt();

                grid_node_size / c
            })
            .min_by(f32::total_cmp)
    }

    fn limit_time_step_by_isolated_particles(&self, grid_node_size: f32) -> Option<f32> {
        profile!("limit_time_step_by_isolated_particles");
        self.particles
            .parameters
            .par_iter()
            .zip(&self.particles.position_gradients)
            .zip(&self.particles.flags)
            .filter_map(|(e, flags)| (!flags.contains(ParticleFlags::TOMBSTONED)).then_some(e))
            .map(|(parameters, position_gradient)| {
                // TODO: do someting with viscosity?
                match parameters.specific {
                    SpecificParticleParameters::Solid {
                        mu,
                        lambda,
                        sand_alpha: _,
                    } => {
                        // Stability analysis of explicit MPM, Technical document 3.12
                        let xi = 3. / grid_node_size / grid_node_size;
                        const R: f32 = 1.; // APIC & CPIC
                        const K: f32 = 1.; // CPIC
                        const D: f32 = 3.; // 3D
                        (parameters.mass
                            / (parameters.initial_volume
                                * xi
                                * (R - K / 2.)
                                * (mu + D / 2. * lambda)))
                            .sqrt()
                    }
                    SpecificParticleParameters::Fluid {
                        exponent,
                        bulk_modulus,
                    } => {
                        // Effective time step restrictions for explicit MPM simulation,
                        // Technical document "Simple bounds"
                        let initial_density = parameters.mass / parameters.initial_volume;
                        let j = position_gradient.determinant();
                        const K: f32 = 6.; // quadratic splines
                        const D: f32 = 3.; // 3D
                        let first = partial_elastic_energy_inviscid_by_invariant_3(
                            bulk_modulus,
                            exponent,
                            j,
                        );
                        if (j - 1.).abs() > SINGULAR_VALUE_SEPARATION {
                            return grid_node_size / j
                                * (initial_density * (j - 1.) / (K * first * D)).sqrt();
                        }

                        let second = double_partial_elastic_energy_inviscid_by_invariant_3(
                            bulk_modulus,
                            exponent,
                            j,
                        );

                        grid_node_size * (initial_density / (K * second * D)).sqrt()
                    }
                }
            })
            .min_by(f32::total_cmp)
    }

    // At least somewhat similar to
    // Effective time step restrictions for explicit MPM simulation 4.2-4
    pub fn limit_time_step_before_integrate(&mut self, grid_node_size: f32) {
        profile!("limit_time_step_before_integrate");
        self.adaptive_time_step_state.time_step_by_velocity =
            self.limit_time_step_by_velocity(grid_node_size);
        self.adaptive_time_step_state.time_step_by_deformation =
            self.limit_time_step_by_deformation();

        self.adaptive_time_step_state.push_current_limit();
    }

    fn limit_time_step_by_velocity(&self, grid_node_size: f32) -> Option<f32> {
        self.particles
            .velocities
            .par_iter()
            .zip(&self.particles.flags)
            .filter_map(|(e, flags)| (!flags.contains(ParticleFlags::TOMBSTONED)).then_some(e))
            .map(Vector3::norm)
            .max_by(|a, b| a.total_cmp(b))
            .and_then(|max_vel| (max_vel != 0.).then_some(0.5 * grid_node_size / max_vel))
    }

    fn limit_time_step_by_deformation(&self) -> Option<f32> {
        const DELTA: f32 = 0.2;
        self.particles
            .velocity_gradients
            .par_iter()
            .zip(&self.particles.flags)
            .filter_map(|(e, flags)| (!flags.contains(ParticleFlags::TOMBSTONED)).then_some(e))
            .map(|velocity_gradient| {
                velocity_gradient
                    .iter()
                    .map(|e| DELTA / e.abs().max(1e-8))
                    .min_by(f32::total_cmp)
                    .unwrap()
            })
            .min_by(f32::total_cmp)
    }
}
