// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZero;

use squishy_volumes_file_frame::{
    IoState, ParticlesObject, combine_matrices, two_f32_to_u32, u32_to_two_f32,
};
use squishy_volumes_file_input::{BulkAttribute, FrameBulkParticles, Object, ObjectParticles};
use squishy_volumes_util::{AnimatedGlobals, ParticleFlags, SplitParameters, attribute};
use squishy_volumes_xpu::{FrameInput, FrameInputError};

use super::*;

pub struct CpuState {
    pub(crate) time: f64,

    pub(crate) animated_globals: AnimatedGlobals,

    pub(crate) adaptive_time_step_state: AdaptiveTimeStepState,
    pub(crate) phase: Phase,
    pub(crate) particles: Particles,

    pub(crate) frame_input: FrameInput,

    pub(crate) grid_nodes: GridNodes,

    pub(crate) interpolated_input: Option<InterpolatedInput>,
}

impl CpuState {
    pub fn new(
        frame: usize,
        io_state: IoState,
        input_reader: squishy_volumes_file_input::InputReader,
    ) -> Result<Self, Error> {
        let time = io_state.time;
        let animated_globals = io_state.animated_globals;

        let n = input_reader.header().total_particles;

        let mut flags = Vec::with_capacity(n);
        let mut parameters = Vec::with_capacity(n);
        let mut initial_positions = Vec::with_capacity(n);
        let mut goal_positions = Vec::with_capacity(n);
        let mut positions = Vec::with_capacity(n);
        let mut position_gradients = Vec::with_capacity(n);
        let mut velocities = Vec::with_capacity(n);
        let mut velocity_gradients = Vec::with_capacity(n);
        let mut elastic_energies = Vec::with_capacity(n);
        let mut collider_bits = Vec::with_capacity(n);
        for (name, mut object) in io_state.particle_objects {
            let range = &input_reader
                .header()
                .get_particle_object(&name)
                .map_err(|error| {
                    Error::FrameInput(FrameInputError::InputError(error.attach_frame(frame)))
                })?
                .particle_range;
            flags.splice(
                range.clone(),
                object
                    .remove_attribute::<attribute::Flags>()
                    .into_iter()
                    .map(two_f32_to_u32)
                    .map(ParticleFlags),
            );
            collider_bits.splice(
                range.clone(),
                object
                    .remove_attribute::<attribute::ColliderBits>()
                    .into_iter()
                    .map(two_f32_to_u32),
            );

            parameters.splice(
                range.clone(),
                SplitParameters {
                    density: object.remove_attribute::<attribute::Density>(),
                    initial_volume: object.remove_attribute::<attribute::InitialVolumes>(),
                    viscosity_dynamic: object.remove_attribute::<attribute::ViscosityDynamic>(),
                    viscosity_bulk: object.remove_attribute::<attribute::ViscosityBulk>(),
                    youngs_modulus: object.remove_attribute::<attribute::YoungsModulus>(),
                    poissons_ratio: object.remove_attribute::<attribute::PoissonsRatio>(),
                    sand_alpha: object.remove_attribute::<attribute::SandAlpha>(),
                    bulk_modulus: object.remove_attribute::<attribute::BulkModulus>(),
                    exponent: object.remove_attribute::<attribute::Exponent>(),
                }
                .to_zipped(),
            );

            initial_positions[range.clone()].copy_from_slice(bytemuck::cast_slice(
                &object.remove_attribute::<attribute::InitialPosition>(),
            ));
            goal_positions[range.clone()].copy_from_slice(bytemuck::cast_slice(
                &object.remove_attribute::<attribute::InitialPosition>(),
            ));
            positions[range.clone()].copy_from_slice(bytemuck::cast_slice(
                &object.remove_attribute::<attribute::InitialPosition>(),
            ));
            position_gradients.splice(
                range.clone(),
                combine_matrices([
                    object.remove_attribute::<attribute::PositionGradientX>(),
                    object.remove_attribute::<attribute::PositionGradientY>(),
                    object.remove_attribute::<attribute::PositionGradientZ>(),
                ])
                .map(Into::into),
            );
            velocities[range.clone()].copy_from_slice(bytemuck::cast_slice(
                &object.remove_attribute::<attribute::Velocity>(),
            ));
            velocity_gradients.splice(
                range.clone(),
                combine_matrices([
                    object.remove_attribute::<attribute::VelocityGradientX>(),
                    object.remove_attribute::<attribute::VelocityGradientY>(),
                    object.remove_attribute::<attribute::VelocityGradientZ>(),
                ])
                .map(Into::into),
            );
            elastic_energies[range.clone()]
                .copy_from_slice(&object.remove_attribute::<attribute::ElasticEnergy>());
        }

        let particles = Particles {
            sort_map: (0..n as u32).collect(),
            reverse_sort_map: (0..n as u32).collect(),

            flags,
            parameters,
            initial_positions,
            goal_positions,
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
            elastic_energies,
            collider_bits,
        };

        let frame_input = FrameInput::new(input_reader, io_state.collider, frame)?;

        Ok(Self {
            time,
            animated_globals,

            particles,

            frame_input,

            phase: Default::default(),
            adaptive_time_step_state: Default::default(),
            grid_nodes: Default::default(),
            interpolated_input: Default::default(),
        })
    }

    pub fn to_io_state(&self, store_bvh: bool, store_grid: bool) -> Result<IoState, Error> {
        let time = self.time;
        let animated_globals = self.animated_globals;

        let Particles {
            sort_map: _,
            reverse_sort_map,

            flags,
            parameters,
            initial_positions,
            goal_positions,
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
            elastic_energies,
            collider_bits,
        } = &self.particles;

        let particle_objects = self
            .frame_input
            .header()
            .objects
            .iter()
            .filter_map(|(name, object)| {
                let Object::Particles(ObjectParticles {
                    particle_range,
                    alembic_attributes,
                }) = object
                else {
                    return None;
                };
                let it = || {
                    particle_range
                        .clone()
                        .into_iter()
                        .map(|i| reverse_sort_map[i] as usize)
                };

                let mut particles = ParticlesObject {
                    num_particles: particle_range.len(),
                    alembic_attributes: alembic_attributes.clone(),
                    attributes: Default::default(),
                };
                // TODO: multi threading
                particles.insert_attribute::<attribute::ElasticEnergy>(
                    it().map(|i| elastic_energies[i]).collect(),
                );
                particles.insert_attribute::<attribute::Flags>(
                    it().map(|i| flags[i].0).map(u32_to_two_f32).collect(),
                );
                particles.insert_attribute::<attribute::ColliderBits>(
                    it().map(|i| collider_bits[i]).map(u32_to_two_f32).collect(),
                );
                particles.insert_attribute::<attribute::Density>(
                    it().map(|i| parameters[i].density).collect(),
                );
                particles.insert_attribute::<attribute::InitialVolumes>(
                    it().map(|i| parameters[i].initial_volume).collect(),
                );
                particles.insert_attribute::<attribute::ViscosityDynamic>(
                    it().map(|i| parameters[i].viscosity_dynamic).collect(),
                );
                particles.insert_attribute::<attribute::ViscosityBulk>(
                    it().map(|i| parameters[i].viscosity_bulk).collect(),
                );
                particles.insert_attribute::<attribute::YoungsModulus>(
                    it().map(|i| parameters[i].youngs_modulus).collect(),
                );
                particles.insert_attribute::<attribute::PoissonsRatio>(
                    it().map(|i| parameters[i].poissons_ratio).collect(),
                );
                particles.insert_attribute::<attribute::SandAlpha>(
                    it().map(|i| parameters[i].sand_alpha).collect(),
                );
                particles.insert_attribute::<attribute::BulkModulus>(
                    it().map(|i| parameters[i].bulk_modulus).collect(),
                );
                particles.insert_attribute::<attribute::Exponent>(
                    it().map(|i| parameters[i].bulk_modulus as f32).collect(),
                );
                particles.insert_attribute::<attribute::Position>(
                    it().map(|i| positions[i].into()).collect(),
                );
                particles.insert_attribute::<attribute::PositionGradientX>(
                    it().map(|i| position_gradients[i].column(0).into())
                        .collect(),
                );
                particles.insert_attribute::<attribute::PositionGradientY>(
                    it().map(|i| position_gradients[i].column(1).into())
                        .collect(),
                );
                particles.insert_attribute::<attribute::PositionGradientZ>(
                    it().map(|i| position_gradients[i].column(2).into())
                        .collect(),
                );
                particles.insert_attribute::<attribute::Velocity>(
                    it().map(|i| velocities[i].into()).collect(),
                );
                particles.insert_attribute::<attribute::VelocityGradientX>(
                    it().map(|i| velocity_gradients[i].column(0).into())
                        .collect(),
                );
                particles.insert_attribute::<attribute::VelocityGradientY>(
                    it().map(|i| velocity_gradients[i].column(1).into())
                        .collect(),
                );
                particles.insert_attribute::<attribute::VelocityGradientZ>(
                    it().map(|i| velocity_gradients[i].column(2).into())
                        .collect(),
                );
                particles.insert_attribute::<attribute::InitialPosition>(
                    it().map(|i| initial_positions[i].into()).collect(),
                );
                particles.insert_attribute::<attribute::GoalPosition>(
                    it().map(|i| goal_positions[i].into()).collect(),
                );

                Some((name.clone(), particles))
            })
            .collect();

        let collider = self.frame_input.collider_start().to_io_collider();

        let grid_nodes = store_grid.then(|| {
            let (node_ids, collider_bits) = self
                .grid_nodes
                .keys
                .iter()
                .map(
                    |GridKey {
                         node_id,
                         collider_bits,
                     }|
                     -> ([i32; 3], u32) { (*node_id.as_ref(), *collider_bits) },
                )
                .unzip();
            let masses = self.grid_nodes.masses.clone();
            let velocites = bytemuck::cast_slice(&self.grid_nodes.velocities).to_vec();

            squishy_volumes_file_frame::GridNodes {
                node_ids,
                collider_bits,
                masses,
                velocites,
            }
        });

        let bvh = store_bvh.then(|| self.frame_input.bvh().clone());

        Ok(IoState {
            time,
            animated_globals,
            particle_objects,
            collider,
            bvh,
            grid_nodes,
        })
    }
}

pub struct CpuRunParameters {
    pub target_time: f64,
    pub max_time_step: f32,
    pub adaptive_time_steps: bool,
    pub store_bvh: bool,
    pub store_grid: bool,
}

impl CpuState {
    pub fn produce_next_state(
        &mut self,
        harness: &squishy_volumes_xpu::Harness,
        CpuRunParameters {
            target_time,
            max_time_step,
            adaptive_time_steps,
            store_bvh,
            store_grid,
        }: CpuRunParameters,
    ) -> Result<(squishy_volumes_file_frame::IoState, Result<(), Error>), Error> {
        squishy_volumes_util::profile!("produce_next_state");

        let harness = harness.scope(
            "Simulation Milliseconds to Next Frame".to_string(),
            NonZero::new(
                ((1000. * self.frame_input.header().consts.seconds_per_frame()) as usize).max(1),
            )
            .unwrap(),
        )?;

        self.adaptive_time_step_state.max_time_step = max_time_step;

        while self.time < target_time {
            harness.check()?;

            if self.adaptive_time_step_state.allowed_time_step() == 0. {
                return Err(Error::ZeroTimeStep);
            }

            let run_phase = adaptive_time_steps
                || (self.phase != Phase::LimitTimeStepBeforeForce
                    && self.phase != Phase::LimitTimeStepBeforeIntegrate);
            if run_phase {
                match self.run_phase() {
                    error @ Err(Error::EnergyError(energy_error)) => {
                        tracing::warn!(?energy_error, "Encountered an energy error.");
                        return Ok((self.to_io_state(store_bvh, store_grid)?, error));
                    }
                    x => x?,
                }
            }

            self.phase = self.phase.cycle();
            if self.phase == Default::default() {
                self.time += self.adaptive_time_step_state.allowed_time_step() as f64;
            }
            harness.step_to(
                ((self.time % self.frame_input.header().consts.seconds_per_frame()) * 1000.)
                    as usize,
            )?;
        }

        if let Some(next_input_frame) = self.frame_input.next_input_frame() {
            self.animated_globals = next_input_frame.animated_globals;
            for bulk in next_input_frame.bulk {
                if let BulkAttribute::Particles(attr) = bulk.meta.captured_attribute {
                    let ObjectParticles { particle_range, .. } = self
                        .frame_input
                        .header()
                        .get_particle_object(&bulk.meta.object_name)
                        .map_err(|error| error.attach_frame(self.frame_input.frame() + 1))
                        .map_err(FrameInputError::InputError)?;
                    match attr {
                        FrameBulkParticles::IsSolid => todo!(),
                        FrameBulkParticles::IsFluid => todo!(),
                        FrameBulkParticles::UseViscosity => todo!(),
                        FrameBulkParticles::UseSandAlpha => todo!(),
                        FrameBulkParticles::IsActive => todo!(),

                        FrameBulkParticles::HasGoal => {
                            let bits: &[bool] = bulk
                                .data
                                .assume_bools()
                                .map_err(|error| {
                                    error
                                        .attach_attr(bulk.meta.captured_attribute)
                                        .attach_name(bulk.meta.object_name.clone())
                                        .attach_frame(self.frame_input.frame() + 1)
                                })
                                .map_err(FrameInputError::InputError)?;
                            for (i, bit) in particle_range.clone().into_iter().zip(bits) {
                                let index = self.particles.reverse_sort_map[i];
                                self.particles.flags[index as usize]
                                    .set(ParticleFlags::HAS_GOAL, *bit);
                            }
                        }

                        FrameBulkParticles::ColliderBits => todo!(),
                        FrameBulkParticles::Transforms => todo!(),
                        FrameBulkParticles::Sizes => todo!(),
                        FrameBulkParticles::Densities => todo!(),
                        FrameBulkParticles::YoungsModuluses => todo!(),
                        FrameBulkParticles::PoissonsRatios => todo!(),
                        FrameBulkParticles::InitialPositions => todo!(),
                        FrameBulkParticles::InitialVelocity => todo!(),
                        FrameBulkParticles::ViscosityDynamic => todo!(),
                        FrameBulkParticles::ViscosityBulk => todo!(),
                        FrameBulkParticles::Exponent => todo!(),
                        FrameBulkParticles::BulkModulus => todo!(),
                        FrameBulkParticles::SandAlpha => todo!(),

                        FrameBulkParticles::GoalPositions => {
                            todo!()
                        }
                    }
                }
            }
        }

        self.frame_input.load_next()?;

        Ok((self.to_io_state(store_bvh, store_grid)?, Ok(())))
    }
}
