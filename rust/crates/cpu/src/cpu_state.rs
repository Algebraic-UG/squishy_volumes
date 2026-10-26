// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{collections::BTreeMap, num::NonZero};

use squishy_volumes_file_frame::{
    IoState, IoStateError, ParticlesObject, combine_matrices, two_f32_to_u32, u32_to_two_f32,
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
        mut io_state: IoState,
        input_reader: squishy_volumes_file_input::InputReader,
    ) -> Result<Self, Error> {
        let time = io_state.time;
        let animated_globals = io_state.animated_globals;

        let n = input_reader.header().total_particles;

        let flags = io_state
            .collect::<attribute::Flags>()?
            .map(two_f32_to_u32)
            .map(ParticleFlags)
            .collect();
        let collider_bits = io_state
            .collect::<attribute::ColliderBits>()?
            .map(two_f32_to_u32)
            .collect();
        let parameters = SplitParameters {
            density: io_state.collect::<attribute::Density>()?.collect(),
            initial_size: io_state.collect::<attribute::InitialSize>()?.collect(),
            viscosity_dynamic: io_state.collect::<attribute::ViscosityDynamic>()?.collect(),
            viscosity_bulk: io_state.collect::<attribute::ViscosityBulk>()?.collect(),
            youngs_modulus: io_state.collect::<attribute::YoungsModulus>()?.collect(),
            poissons_ratio: io_state.collect::<attribute::PoissonsRatio>()?.collect(),
            sand_alpha: io_state.collect::<attribute::SandAlpha>()?.collect(),
            bulk_modulus: io_state.collect::<attribute::BulkModulus>()?.collect(),
            exponent: io_state.collect::<attribute::Exponent>()?.collect(),
        }
        .to_zipped()
        .collect();
        let initial_positions = io_state
            .collect::<attribute::InitialPosition>()?
            .map(Into::into)
            .collect();
        let goal_positions = io_state
            .collect::<attribute::GoalPosition>()?
            .map(Into::into)
            .collect();
        let positions = io_state
            .collect::<attribute::Position>()?
            .map(Into::into)
            .collect();
        let position_gradients = combine_matrices(
            io_state.collect::<attribute::PositionGradientX>()?,
            io_state.collect::<attribute::PositionGradientY>()?,
            io_state.collect::<attribute::PositionGradientZ>()?,
        )
        .map(Into::into)
        .collect();
        let velocities = io_state
            .collect::<attribute::Velocity>()?
            .map(Into::into)
            .collect();
        let velocity_gradients = combine_matrices(
            io_state.collect::<attribute::VelocityGradientX>()?,
            io_state.collect::<attribute::VelocityGradientY>()?,
            io_state.collect::<attribute::VelocityGradientZ>()?,
        )
        .map(Into::into)
        .collect();
        let elastic_energies = io_state.collect::<attribute::ElasticEnergy>()?.collect();

        for p in io_state.particle_objects.values() {
            if !p.attributes.is_empty() {
                tracing::error!(?p.attributes);
                panic!("Missed attrbiutes");
            }
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
                if let Object::Particles(object) = object {
                    Some((name, object))
                } else {
                    None
                }
            })
            .map(|(name, object)| {
                let ObjectParticles {
                    particle_range,
                    alembic_attributes,
                } = object;
                struct Permuter<I> {
                    iterator: I,
                }
                impl<I: Iterator<Item = usize> + Clone> Permuter<I> {
                    fn it<'a, T>(&self, to_permute: &'a [T]) -> impl Iterator<Item = &'a T> {
                        self.iterator.clone().map(|i| &to_permute[i])
                    }
                }
                let p = Permuter {
                    iterator: particle_range
                        .clone()
                        .into_iter()
                        .map(|i| reverse_sort_map[i] as usize),
                };

                let mut ps = ParticlesObject {
                    num_particles: particle_range.len(),
                    alembic_attributes: alembic_attributes.clone(),
                    attributes: Default::default(),
                };
                // TODO: multi threading
                use attribute as a;
                (|| {
                    ps.insert_attr::<a::ElasticEnergy>(p.it(elastic_energies).cloned())?;

                    ps.insert_attr::<a::Flags>(p.it(flags).map(|flag| u32_to_two_f32(flag.0)))?;
                    ps.insert_attr::<a::ColliderBits>(
                        p.it(collider_bits).cloned().map(u32_to_two_f32),
                    )?;

                    ps.insert_attr::<a::Density>(p.it(parameters).map(|p| p.density))?;
                    ps.insert_attr::<a::InitialSize>(p.it(parameters).map(|p| p.initial_size))?;
                    ps.insert_attr::<a::ViscosityDynamic>(
                        p.it(parameters).map(|p| p.viscosity_dynamic),
                    )?;
                    ps.insert_attr::<a::ViscosityBulk>(p.it(parameters).map(|p| p.viscosity_bulk))?;
                    ps.insert_attr::<a::YoungsModulus>(p.it(parameters).map(|p| p.youngs_modulus))?;
                    ps.insert_attr::<a::PoissonsRatio>(p.it(parameters).map(|p| p.poissons_ratio))?;
                    ps.insert_attr::<a::SandAlpha>(p.it(parameters).map(|p| p.sand_alpha))?;
                    ps.insert_attr::<a::BulkModulus>(p.it(parameters).map(|p| p.bulk_modulus))?;
                    ps.insert_attr::<a::Exponent>(p.it(parameters).map(|p| p.exponent as f32))?;

                    ps.insert_attr::<a::Position>(p.it(positions).cloned().map(Into::into))?;
                    ps.insert_attr::<a::PositionGradientX>(
                        p.it(position_gradients).map(|p| p.column(0).into()),
                    )?;
                    ps.insert_attr::<a::PositionGradientY>(
                        p.it(position_gradients).map(|p| p.column(1).into()),
                    )?;
                    ps.insert_attr::<a::PositionGradientZ>(
                        p.it(position_gradients).map(|p| p.column(2).into()),
                    )?;

                    ps.insert_attr::<a::Velocity>(p.it(velocities).cloned().map(Into::into))?;
                    ps.insert_attr::<a::VelocityGradientX>(
                        p.it(velocity_gradients).map(|p| p.column(0).into()),
                    )?;
                    ps.insert_attr::<a::VelocityGradientY>(
                        p.it(velocity_gradients).map(|p| p.column(1).into()),
                    )?;
                    ps.insert_attr::<a::VelocityGradientZ>(
                        p.it(velocity_gradients).map(|p| p.column(2).into()),
                    )?;

                    ps.insert_attr::<a::InitialPosition>(
                        p.it(initial_positions).cloned().map(Into::into),
                    )?;
                    ps.insert_attr::<a::GoalPosition>(
                        p.it(goal_positions).cloned().map(Into::into),
                    )?;
                    Ok(())
                })()
                .map_err(|error| IoStateError::ParticleObject {
                    name: name.clone(),
                    error,
                })?;

                Ok::<_, IoStateError>((name.clone(), ps))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

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
                                .assume_bools()
                                .map_err(|error| error.attach_frame(self.frame_input.frame() + 1))
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
