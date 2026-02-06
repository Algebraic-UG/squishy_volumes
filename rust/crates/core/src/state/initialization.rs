// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    collections::BTreeMap,
    num::NonZero,
    sync::{Arc, atomic::AtomicBool},
};

use bitflags::bitflags_match;
use nalgebra::{Matrix3, Matrix4, Vector3};
use squishy_volumes_api::T;
use thiserror::Error;
use tracing::info;

use crate::{
    ParticleFlags, Report, ReportInfo,
    elastic::{lambda_stable_neo_hookean, mu_stable_neo_hookean},
    input_file::{InputFrame, InputHeader, InputObjectType, ParticlesInput},
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

        for object in input_header.objects {
            match object.ty {
                InputObjectType::Particles => {
                    let object_index = ObjectIndex::Particles(particle_objects.len());
                    name_map.insert(object.name.clone(), object_index);

                    particle_objects.push(ObjectParticles::default());
                    let particle_object = particle_objects.last_mut().unwrap();

                    let Some(ParticlesInput {
                        flags,
                        transforms,
                        sizes,
                        densities,
                        youngs_moduluses,
                        poissons_ratios,
                        initial_positions,
                        initial_velocities,
                        viscosities_dynamic,
                        viscosities_bulk,
                        exponents,
                        bulk_moduluses,
                        sand_alphas,
                    }) = first_frame.particles_input.get(&object.name)
                    else {
                        continue;
                    };

                    let first_index = particles.sort_map.len();

                    let (positions, position_gradients): (Vec<Vector3<T>>, Vec<Matrix3<T>>) =
                        transforms
                            .chunks_exact(16)
                            .map(Matrix4::from_column_slice)
                            .map(|transform| -> (Vector3<T>, Matrix3<T>) {
                                (
                                    Vector3::new(transform.m14, transform.m24, transform.m34),
                                    transform.fixed_view::<3, 3>(0, 0).into(),
                                )
                            })
                            .unzip();
                    particles.positions.extend(positions.into_iter());
                    particles
                        .position_gradients
                        .extend(position_gradients.into_iter());

                    let initial_volumes = sizes.iter().map(|size| size.powi(3));
                    particles.initial_volumes.extend(initial_volumes.clone());
                    particles.masses.extend(
                        densities
                            .iter()
                            .zip(initial_volumes)
                            .map(|(density, volume)| density * volume),
                    );

                    particles.initial_positions.extend(
                        initial_positions
                            .chunks_exact(3)
                            .map(Vector3::from_column_slice),
                    );

                    for (i, flags) in flags.iter().enumerate() {
                        let flags = ParticleFlags(*flags);

                        let mu = mu_stable_neo_hookean(youngs_moduluses[i], poissons_ratios[i]);
                        let lambda =
                            lambda_stable_neo_hookean(youngs_moduluses[i], poissons_ratios[i]);
                        let exponent = exponents[i];
                        let bulk_modulus = bulk_moduluses[i];

                        let viscosity = flags.contains(ParticleFlags::UseViscosity).then_some(
                            ViscosityParameters {
                                dynamic: viscosities_dynamic[i],
                                bulk: viscosities_bulk[i],
                            },
                        );
                        let sand_alpha = flags
                            .contains(ParticleFlags::UseSandAlpha)
                            .then_some(sand_alphas[i]);
                        particles
                            .parameters
                            .push(bitflags_match!(flags, {
                                ParticleFlags::IsSolid => Ok(ParticleParameters::Solid { mu, lambda, viscosity, sand_alpha }),
                                ParticleFlags::IsFluid => Ok(ParticleParameters::Fluid { exponent, bulk_modulus, viscosity }),
                                _=> Err(StateInitializationError::ParticleFlagsInvalid(i)),
                            })?);
                    }

                    particle_object.particles = (first_index..particles.sort_map.len()).collect();
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
            particles,
            grid_momentum,
            grid_collider_distances,
            grid_collider_momentums,
        })
    }
}
