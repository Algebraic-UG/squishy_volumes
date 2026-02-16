// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    collections::BTreeMap,
    iter::repeat_n,
    num::NonZero,
    sync::{Arc, atomic::AtomicBool},
};

use nalgebra::{Matrix3, Matrix4, Vector3};
use squishy_volumes_api::T;
use thiserror::Error;
use tracing::info;

use crate::{
    ParticleFlags, Report, ReportInfo,
    elastic::{lambda_stable_neo_hookean, mu_stable_neo_hookean},
    input_file::{InputFrame, InputHeader, InputObject},
    state::{
        ObjectIndex,
        object::ObjectParticles,
        particles::{ParticleParameters, Particles, ViscosityParameters},
    },
};

use super::State;

#[derive(Error, Debug)]
pub enum StateInitializationError {
    #[error("the flags for a particle {0} are invalid")]
    ParticleFlagsInvalid(usize),
    #[error("There was no input for particle object {0}")]
    ParticleInputMissing(String),
}

impl State {
    pub fn new(
        run: Arc<AtomicBool>,
        report: Report,
        input_header: InputHeader,
        first_frame: InputFrame,
    ) -> Result<Self, StateInitializationError> {
        info!("Creating new simulation state from first input frame");

        let report = report.new_sub(ReportInfo {
            name: "Initializing Objects".to_string(),
            completed_steps: 0,
            steps_to_completion: NonZero::new(input_header.objects.len().max(1)).unwrap(),
        });

        let mut name_map = BTreeMap::new();
        let mut particles = Particles::default();
        let mut particle_objects = Vec::new();
        let mut collider_objects = Vec::new();

        let Particles {
            sort_map,
            reverse_sort_map,
            states,
            parameters,
            masses,
            initial_volumes,
            initial_positions,
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
            elastic_energies,
            collider_insides,
            trial_position_gradients: _,
            action_matrices: _,
        } = &mut particles;

        for (name, object) in input_header.objects.iter() {
            match object {
                InputObject::Particles => {
                    let object_index = ObjectIndex::Particles(particle_objects.len());
                    name_map.insert(name.clone(), object_index);

                    particle_objects.push(ObjectParticles::default());
                    let particle_object = particle_objects.last_mut().unwrap();

                    let input = first_frame.particles_inputs.get(name).ok_or_else(|| {
                        StateInitializationError::ParticleInputMissing(name.clone())
                    })?;

                    let first_index = sort_map.len();

                    let n = input.flags.len();

                    sort_map.extend(first_index..first_index + n);
                    reverse_sort_map.extend(first_index..first_index + n);
                    states.extend(repeat_n(Default::default(), n));
                    collider_insides.extend(repeat_n(Default::default(), n));

                    let (input_positions, input_position_gradients): (
                        Vec<Vector3<T>>,
                        Vec<Matrix3<T>>,
                    ) = input
                        .transforms
                        .chunks_exact(16)
                        .map(Matrix4::from_column_slice)
                        .map(|transform| -> (Vector3<T>, Matrix3<T>) {
                            (
                                Vector3::new(transform.m14, transform.m24, transform.m34),
                                transform.fixed_view::<3, 3>(0, 0).into(),
                            )
                        })
                        .unzip();
                    positions.extend(input_positions.into_iter());
                    position_gradients.extend(input_position_gradients.into_iter());

                    velocities.extend(
                        input
                            .initial_velocities
                            .chunks_exact(3)
                            .map(Vector3::from_column_slice),
                    );

                    let input_initial_volumes = input.sizes.iter().map(|size| size.powi(3));
                    initial_volumes.extend(input_initial_volumes.clone());
                    masses.extend(
                        input
                            .densities
                            .iter()
                            .zip(input_initial_volumes)
                            .map(|(density, volume)| density * volume),
                    );

                    initial_positions.extend(
                        input
                            .initial_positions
                            .chunks_exact(3)
                            .map(Vector3::from_column_slice),
                    );

                    // TODO:

                    velocity_gradients.extend(repeat_n(Matrix3::zeros(), n));
                    elastic_energies.extend(repeat_n(0., n));

                    for (i, flags) in input.flags.iter().enumerate() {
                        let flags = ParticleFlags(*flags);

                        let mu = mu_stable_neo_hookean(
                            input.youngs_moduluses[i],
                            input.poissons_ratios[i],
                        );
                        let lambda = lambda_stable_neo_hookean(
                            input.youngs_moduluses[i],
                            input.poissons_ratios[i],
                        );
                        let exponent = input.exponents[i];
                        let bulk_modulus = input.bulk_moduluses[i];

                        let viscosity = flags.contains(ParticleFlags::UseViscosity).then_some(
                            ViscosityParameters {
                                dynamic: input.viscosities_dynamic[i],
                                bulk: input.viscosities_bulk[i],
                            },
                        );
                        let sand_alpha = flags
                            .contains(ParticleFlags::UseSandAlpha)
                            .then_some(input.sand_alphas[i]);

                        parameters.push(if flags.contains(ParticleFlags::IsSolid) {
                            ParticleParameters::Solid {
                                mu,
                                lambda,
                                viscosity,
                                sand_alpha,
                            }
                        } else if flags.contains(ParticleFlags::IsFluid) {
                            ParticleParameters::Fluid {
                                exponent,
                                bulk_modulus,
                                viscosity,
                            }
                        } else {
                            Err(StateInitializationError::ParticleFlagsInvalid(i))?
                        });
                    }

                    particle_object.particles = (first_index..first_index + n).collect();
                }
                InputObject::Collider { .. } => {
                    let object_index = ObjectIndex::Collider(particle_objects.len());
                    name_map.insert(name.clone(), object_index);
                }
            }

            report.step();
        }

        let time = 0.;
        let phase = Default::default();

        let grid_momentum = Default::default();
        let grid_collider_distances = Default::default();
        let grid_collider_momentums = Default::default();

        Ok(Self {
            time,
            phase,
            name_map,
            particle_objects,
            collider_objects,
            particles,
            grid_momentum,
            grid_collider_distances,
            grid_collider_momentums,
            interpolated_input: None,
        })
    }
}
