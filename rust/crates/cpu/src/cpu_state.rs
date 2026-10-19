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

        let mut particle_objects = io_state.particle_objects;
        let flags = particle_objects
            .values_mut()
            .flat_map(|p| p.remove_attr::<attribute::Flags>().into_iter())
            .map(two_f32_to_u32)
            .map(ParticleFlags)
            .collect();
        let collider_bits = particle_objects
            .values_mut()
            .flat_map(|p| p.remove_attr::<attribute::ColliderBits>())
            .map(two_f32_to_u32)
            .collect();
        let parameters = particle_objects
            .values_mut()
            .flat_map(|p| {
                SplitParameters {
                    density: p.remove_attr::<attribute::Density>(),
                    initial_volume: p.remove_attr::<attribute::InitialVolumes>(),
                    viscosity_dynamic: p.remove_attr::<attribute::ViscosityDynamic>(),
                    viscosity_bulk: p.remove_attr::<attribute::ViscosityBulk>(),
                    youngs_modulus: p.remove_attr::<attribute::YoungsModulus>(),
                    poissons_ratio: p.remove_attr::<attribute::PoissonsRatio>(),
                    sand_alpha: p.remove_attr::<attribute::SandAlpha>(),
                    bulk_modulus: p.remove_attr::<attribute::BulkModulus>(),
                    exponent: p.remove_attr::<attribute::Exponent>(),
                }
                .to_zipped()
            })
            .collect();
        let initial_positions = particle_objects
            .values_mut()
            .flat_map(|p| p.remove_attr::<attribute::InitialPosition>().into_iter())
            .map(Into::into)
            .collect();
        let goal_positions = particle_objects
            .values_mut()
            .flat_map(|p| p.remove_attr::<attribute::GoalPosition>().into_iter())
            .map(Into::into)
            .collect();
        let positions = particle_objects
            .values_mut()
            .flat_map(|p| p.remove_attr::<attribute::Position>().into_iter())
            .map(Into::into)
            .collect();
        let position_gradients = particle_objects
            .values_mut()
            .flat_map(|p| {
                combine_matrices([
                    p.remove_attr::<attribute::PositionGradientX>(),
                    p.remove_attr::<attribute::PositionGradientY>(),
                    p.remove_attr::<attribute::PositionGradientZ>(),
                ])
                .map(Into::into)
            })
            .collect();
        let velocities = particle_objects
            .values_mut()
            .flat_map(|p| p.remove_attr::<attribute::Velocity>().into_iter())
            .map(Into::into)
            .collect();
        let velocity_gradients = particle_objects
            .values_mut()
            .flat_map(|p| {
                combine_matrices([
                    p.remove_attr::<attribute::VelocityGradientX>(),
                    p.remove_attr::<attribute::VelocityGradientY>(),
                    p.remove_attr::<attribute::VelocityGradientZ>(),
                ])
                .map(Into::into)
            })
            .collect();
        let elastic_energies = particle_objects
            .values_mut()
            .flat_map(|p| p.remove_attr::<attribute::ElasticEnergy>().into_iter())
            .collect();

        for p in particle_objects.values() {
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
                let Object::Particles(ObjectParticles {
                    particle_range,
                    alembic_attributes,
                }) = object
                else {
                    return None;
                };
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
                ps.insert_attr::<a::ElasticEnergy>(p.it(elastic_energies).cloned());

                ps.insert_attr::<a::Flags>(p.it(flags).map(|flag| u32_to_two_f32(flag.0)));
                ps.insert_attr::<a::ColliderBits>(p.it(collider_bits).cloned().map(u32_to_two_f32));

                ps.insert_attr::<a::Density>(p.it(parameters).map(|p| p.density));
                ps.insert_attr::<a::InitialVolumes>(p.it(parameters).map(|p| p.initial_volume));
                ps.insert_attr::<a::ViscosityDynamic>(
                    p.it(parameters).map(|p| p.viscosity_dynamic),
                );
                ps.insert_attr::<a::ViscosityBulk>(p.it(parameters).map(|p| p.viscosity_bulk));
                ps.insert_attr::<a::YoungsModulus>(p.it(parameters).map(|p| p.youngs_modulus));
                ps.insert_attr::<a::PoissonsRatio>(p.it(parameters).map(|p| p.poissons_ratio));
                ps.insert_attr::<a::SandAlpha>(p.it(parameters).map(|p| p.sand_alpha));
                ps.insert_attr::<a::BulkModulus>(p.it(parameters).map(|p| p.bulk_modulus));
                ps.insert_attr::<a::Exponent>(p.it(parameters).map(|p| p.exponent as f32));

                ps.insert_attr::<a::Position>(p.it(positions).cloned().map(Into::into));
                ps.insert_attr::<a::PositionGradientX>(
                    p.it(position_gradients).map(|p| p.column(0).into()),
                );
                ps.insert_attr::<a::PositionGradientY>(
                    p.it(position_gradients).map(|p| p.column(1).into()),
                );
                ps.insert_attr::<a::PositionGradientZ>(
                    p.it(position_gradients).map(|p| p.column(2).into()),
                );

                ps.insert_attr::<a::Velocity>(p.it(velocities).cloned().map(Into::into));
                ps.insert_attr::<a::VelocityGradientX>(
                    p.it(velocity_gradients).map(|p| p.column(0).into()),
                );
                ps.insert_attr::<a::VelocityGradientY>(
                    p.it(velocity_gradients).map(|p| p.column(1).into()),
                );
                ps.insert_attr::<a::VelocityGradientZ>(
                    p.it(velocity_gradients).map(|p| p.column(2).into()),
                );

                ps.insert_attr::<a::InitialPosition>(
                    p.it(initial_positions).cloned().map(Into::into),
                );
                ps.insert_attr::<a::GoalPosition>(p.it(goal_positions).cloned().map(Into::into));

                Some((name.clone(), ps))
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
