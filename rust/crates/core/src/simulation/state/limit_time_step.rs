// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use nalgebra::{Matrix3, Vector3};
use squishy_volumes_api::T;

const TIME_STEP_HISTORY_LENGTH: usize = 10;

use crate::{
    math::SINGULAR_VALUE_SEPARATION,
    simulation::{
        elastic::{
            double_partial_elastic_energy_inviscid_by_invariant_3,
            first_piola_stress_inviscid_svd_in_diagonal_space,
            first_piola_stress_neo_hookean_svd_in_diagonal_space,
            partial_elastic_energy_inviscid_by_invariant_3,
            second_derivative_inviscid_svd_in_diagonal_space,
            second_derivative_neo_hookean_svd_in_diagonal_space,
        },
        particles::ParticleParameters,
    },
};

use super::{PhaseInput, State, profile};

impl State {
    pub(super) fn limit_time_step_before_force(self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("limit_time_step_before_force");
        phase_input.time_step_by_sound = self.limit_time_step_by_speed_of_sound(phase_input);
        phase_input.time_step_by_isolated = self.limit_time_step_by_isolated_particles(phase_input);

        apply_limit(phase_input);
        Ok(self)
    }

    // Effective time step restrictions for explicit MPM simulation 4.1 Sound Speed
    fn limit_time_step_by_speed_of_sound(&self, phase_input: &mut PhaseInput) -> Option<T> {
        profile!("limit_time_step_by_speed_of_sound");
        let grid_node_size = phase_input.setup.settings.grid_node_size;
        self.particles
            .parameters
            .iter()
            .zip(self.particles.position_gradients.iter())
            .zip(self.particles.masses.iter())
            .zip(self.particles.initial_volumes.iter())
            .map(
                |(((parameters, position_gradient), mass), initial_volume)| {
                    let s = position_gradient.svd(false, false).singular_values;

                    let j = s.product();

                    // we need to use L'Hôpital in this case
                    let xy_close = (s.x - s.y).abs() < SINGULAR_VALUE_SEPARATION;
                    let yz_close = (s.y - s.z).abs() < SINGULAR_VALUE_SEPARATION;
                    let zx_close = (s.z - s.x).abs() < SINGULAR_VALUE_SEPARATION;

                    let first: Vector3<T>;
                    let second: Matrix3<T>;

                    match parameters {
                        ParticleParameters::Solid {
                            mu,
                            lambda,
                            viscosity: _,  // viscosity is TODO
                            sand_alpha: _, // this shouldn't lower the required timestep
                        } => {
                            first = first_piola_stress_neo_hookean_svd_in_diagonal_space(
                                *mu, *lambda, &s,
                            );
                            second = second_derivative_neo_hookean_svd_in_diagonal_space(
                                *mu, *lambda, &s,
                            );
                        }
                        ParticleParameters::Fluid {
                            exponent,
                            bulk_modulus,
                            viscosity: _, // viscosity is TODO
                        } => {
                            first = first_piola_stress_inviscid_svd_in_diagonal_space(
                                *bulk_modulus,
                                *exponent,
                                &s,
                            );
                            second = second_derivative_inviscid_svd_in_diagonal_space(
                                *bulk_modulus,
                                *exponent,
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
                    .max_by(T::total_cmp)
                    .unwrap()
                        / j;
                    let initial_density = mass / initial_volume;
                    let current_density = initial_density / j;

                    let c = (kappa / current_density).sqrt();

                    grid_node_size / c
                },
            )
            .min_by(T::total_cmp)
    }

    fn limit_time_step_by_isolated_particles(&self, phase_input: &mut PhaseInput) -> Option<T> {
        profile!("limit_time_step_by_isolated_particles");
        let grid_node_size = phase_input.setup.settings.grid_node_size;
        self.particles
            .parameters
            .iter()
            .zip(self.particles.masses.iter())
            .zip(self.particles.initial_volumes.iter())
            .zip(self.particles.position_gradients.iter())
            .map(
                |(((parameters, mass), initial_volume), position_gradient)| {
                    match parameters {
                        ParticleParameters::Solid {
                            mu,
                            lambda,
                            viscosity: _,  // viscosity is TODO
                            sand_alpha: _, // this shouldn't lower the required timestep
                        } => {
                            // Stability analysis of explicit MPM, Technical document 3.12
                            let xi = 3. / grid_node_size / grid_node_size;
                            const R: T = 1.; // APIC & CPIC
                            const K: T = 1.; // CPIC
                            const D: T = 3.; // 3D
                            (mass / (initial_volume * xi * (R - K / 2.) * (mu + D / 2. * lambda)))
                                .sqrt()
                        }
                        ParticleParameters::Fluid {
                            exponent,
                            bulk_modulus,
                            viscosity: _, // viscosity is TODO
                        } => {
                            // Effective time step restrictions for explicit MPM simulation,
                            // Technical document "Simple bounds"
                            let initial_density = mass / initial_volume;
                            let j = position_gradient.determinant();
                            const K: T = 6.; // quadratic splines
                            const D: T = 3.; // 3D
                            let first = partial_elastic_energy_inviscid_by_invariant_3(
                                *bulk_modulus,
                                *exponent,
                                j,
                            );
                            if (j - 1.).abs() > SINGULAR_VALUE_SEPARATION {
                                return grid_node_size / j
                                    * (initial_density * (j - 1.) / (K * first * D)).sqrt();
                            }

                            let second = double_partial_elastic_energy_inviscid_by_invariant_3(
                                *bulk_modulus,
                                *exponent,
                                j,
                            );

                            grid_node_size * (initial_density / (K * second * D)).sqrt()
                        }
                    }
                },
            )
            .min_by(T::total_cmp)
    }

    // At least somewhat similar to
    // Effective time step restrictions for explicit MPM simulation 4.2-4
    pub(super) fn limit_time_step_before_integrate(
        self,
        phase_input: &mut PhaseInput,
    ) -> Result<Self> {
        profile!("limit_time_step_before_integrate");
        let grid_node_size = phase_input.setup.settings.grid_node_size;
        phase_input.time_step_by_velocity = self
            .particles
            .velocities
            .iter()
            .map(Vector3::norm)
            .max_by(|a, b| a.total_cmp(b))
            .map(|max_vel| {
                if max_vel != 0. {
                    0.5 * grid_node_size / max_vel
                } else {
                    phase_input.max_time_step
                }
            });

        const DELTA: T = 0.2;
        phase_input.time_step_by_deformation = self
            .particles
            .velocity_gradients
            .iter()
            .map(|velocity_gradient| {
                velocity_gradient
                    .iter()
                    .map(|e| DELTA / e.abs().max(1e-8))
                    .min_by(T::total_cmp)
                    .unwrap()
            })
            .min_by(T::total_cmp);

        apply_limit(phase_input);
        Ok(self)
    }
}

fn apply_limit(
    PhaseInput {
        max_time_step,
        time_step_by_velocity,
        time_step_by_deformation,
        time_step_by_sound,
        time_step_by_isolated,
        time_step,
        time_step_prior,
        ..
    }: &mut PhaseInput,
) {
    let max_allowed = [
        time_step_by_velocity.unwrap_or(T::MAX),
        time_step_by_deformation.unwrap_or(T::MAX),
        time_step_by_sound.unwrap_or(T::MAX),
        time_step_by_isolated.unwrap_or(T::MAX),
        *max_time_step,
    ]
    .into_iter()
    .min_by(T::total_cmp)
    .unwrap();

    time_step_prior.push_back(max_allowed);
    if time_step_prior.len() > TIME_STEP_HISTORY_LENGTH {
        time_step_prior.pop_front();
    }
    *time_step = time_step_prior
        .iter()
        .cloned()
        .min_by(T::total_cmp)
        .unwrap();
}
