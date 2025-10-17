use anyhow::Result;
use nalgebra::Vector3;
use squishy_volumes_api::T;

const TIME_STEP_HISTORY_LENGTH: usize = 10;

use crate::{
    math::SINGULAR_VALUE_SEPARATION,
    simulation::{
        elastic::{
            first_piola_stress_neo_hookean_svd_in_diagonal_space,
            second_derivative_neo_hookean_svd_in_diagonal_space,
        },
        particles::ParticleParameters,
    },
};

use super::{PhaseInput, State, profile};

impl State {
    // Effective time step restrictions for explicit MPM simulation 4.1 Sound Speed
    // Stability analysis of explicit MPM, Technical document 3.12
    pub(super) fn limit_time_step_before_force(self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("limit_time_step_before_force");
        let grid_node_size = phase_input.setup.settings.grid_node_size;
        phase_input.time_step_by_sound = self
            .particles
            .parameters
            .iter()
            .zip(self.particles.position_gradients.iter())
            .zip(self.particles.masses.iter())
            .zip(self.particles.initial_volumes.iter())
            .map(
                |(((parameters, position_gradient), mass), initial_volume)| {
                    match parameters {
                        ParticleParameters::Solid {
                            mu,
                            lambda,
                            viscosity,
                            sand_alpha,
                        } => {
                            let s = position_gradient.svd(false, false).singular_values;
                            let first = first_piola_stress_neo_hookean_svd_in_diagonal_space(
                                *mu, *lambda, &s,
                            );
                            let second = second_derivative_neo_hookean_svd_in_diagonal_space(
                                *mu, *lambda, &s,
                            );

                            let j = s.product();

                            // we need to use L'Hôpital in this case
                            let xy_close = (s.x - s.y).abs() < SINGULAR_VALUE_SEPARATION;
                            let yz_close = (s.y - s.z).abs() < SINGULAR_VALUE_SEPARATION;
                            let zx_close = (s.z - s.x).abs() < SINGULAR_VALUE_SEPARATION;

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
                        }
                        ParticleParameters::Fluid {
                            exponent,
                            bulk_modulus,
                            viscosity,
                        } => todo!(),
                    }
                },
            )
            .min_by(T::total_cmp);

        phase_input.time_step_by_isolated = self
            .particles
            .parameters
            .iter()
            .zip(self.particles.position_gradients.iter())
            .zip(self.particles.masses.iter())
            .zip(self.particles.initial_volumes.iter())
            .map(
                |(((parameters, position_gradient), mass), initial_volume)| {
                    match parameters {
                        ParticleParameters::Solid {
                            mu,
                            lambda,
                            viscosity,
                            sand_alpha,
                        } => {
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
                            viscosity,
                        } => todo!(),
                    }
                },
            )
            .min_by(T::total_cmp);

        // just for comparison
        phase_input.time_step_by_sound_simple = self
            .particles
            .parameters
            .iter()
            .zip(self.particles.position_gradients.iter())
            .zip(self.particles.masses.iter())
            .zip(self.particles.initial_volumes.iter())
            .map(
                |(((parameters, position_gradient), mass), initial_volume)| match parameters {
                    ParticleParameters::Solid {
                        mu,
                        lambda,
                        viscosity,
                        sand_alpha,
                    } => {
                        let initial_density = mass / initial_volume;
                        let current_density = initial_density / position_gradient.determinant();

                        let bulk_modulus = lambda + 2. / 3. * mu;
                        let c = (bulk_modulus / current_density).sqrt();

                        grid_node_size / c
                    }
                    ParticleParameters::Fluid {
                        exponent,
                        bulk_modulus,
                        viscosity,
                    } => todo!(),
                },
            )
            .min_by(T::total_cmp);

        apply_limit(phase_input);
        Ok(self)
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
