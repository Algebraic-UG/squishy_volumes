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

use std::{collections::BTreeMap, num::NonZero};

use nalgebra::{Matrix4, Vector3};
use squishy_volumes_file_frame::{
    IoState, IoStateError, ParticleObjectError, ParticlesObject, combine_matrices, two_f32_to_u32,
    u32_to_two_f32,
};
use squishy_volumes_file_input::{
    BulkAttribute, FrameBulkParticles, FrameVerificationError, Header, Object, ObjectParticles,
    OwnedFrameBulk,
};
use squishy_volumes_util::{
    AnimatedGlobals, ParticleFlags, ParticleParameters, SplitParameters, attribute,
};
use squishy_volumes_xpu::{FrameInput, SolverError};

use super::*;

pub struct CpuState {
    pub(crate) animated_globals: AnimatedGlobals,

    pub(crate) adaptive_time_step_state: AdaptiveTimeStepState,
    pub(crate) phase: Phase,
    pub(crate) particles: Particles,

    pub(crate) frame_input: FrameInput,

    pub(crate) grid_nodes: GridNodes,

    pub(crate) interpolated_input: Option<InterpolatedInput>,

    pub(crate) last_frame_substeps: u32,
}

impl CpuState {
    pub fn new(
        frame: usize,
        mut io_state: IoState,
        input_reader: squishy_volumes_file_input::InputReader,
    ) -> Result<Self, Error> {
        io_state.convert_to_simulation_units(input_reader.header().consts.simulation_scale);
        let time = io_state.time;
        let animated_globals = io_state.animated_globals;

        let n = input_reader.header().total_particles;
        let particles = Self::read_particles(&mut io_state, n).map_err(SolverError::ReadState)?;

        let adaptive_time_step_state =
            AdaptiveTimeStepState::from_history(&io_state.time_step_history);

        let frame_input = FrameInput::new(input_reader, io_state.collider, frame, time)
            .map_err(SolverError::PrepareInput)?;

        Ok(Self {
            animated_globals,

            particles,

            frame_input,

            phase: Default::default(),
            adaptive_time_step_state,
            grid_nodes: Default::default(),
            interpolated_input: Default::default(),
            last_frame_substeps: 0,
        })
    }

    fn read_particles(io_state: &mut IoState, n: usize) -> Result<Particles, IoStateError> {
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

        Ok(Particles {
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
        })
    }

    pub fn to_io_state(&mut self, store_bvh: bool, store_grid: bool) -> Result<IoState, Error> {
        self.particles.recompute_elastic_energies();
        let time = self.frame_input.time();
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
                    Ok::<_, ParticleObjectError>(())
                })()
                .map_err(|error| error.attach_name(name))?;

                Ok::<_, IoStateError>((name.clone(), ps))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(SolverError::StoreState)?;

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

        let mut io_state = IoState {
            time,
            animated_globals,
            particle_objects,
            collider,
            time_step_history: self.adaptive_time_step_state.history(),
            bvh,
            grid_nodes,
        };
        io_state.convert_to_io_units(self.frame_input.header().consts.simulation_scale);
        Ok(io_state)
    }
}

pub struct CpuRunParameters {
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
        self.adaptive_time_step_state.adaptive_time_steps = adaptive_time_steps;
        self.adaptive_time_step_state.time_step_by_collider = adaptive_time_steps
            .then(|| self.limit_time_step_by_collider())
            .flatten();
        self.last_frame_substeps = 0;

        while !self.frame_input.is_frame_finished() {
            harness.check()?;

            if self.adaptive_time_step_state.allowed_time_step() == 0. {
                return Err(Error::ZeroTimeStep);
            }

            match self.run_phase() {
                error @ Err(Error::Energy(energy_error)) => {
                    tracing::warn!(?energy_error, "Encountered an energy error.");
                    return Ok((self.to_io_state(store_bvh, store_grid)?, error));
                }
                x => x?,
            }

            self.phase = self.phase.cycle();
            if self.phase == Default::default() {
                self.frame_input
                    .advance(self.adaptive_time_step_state.allowed_time_step() as f64);
                self.last_frame_substeps += 1;
            }
            harness.step_to((self.frame_input.seconds_into_frame() * 1000.) as usize)?;
        }

        if let Some(next_input_frame) = self.frame_input.next_input_frame() {
            self.animated_globals = next_input_frame.animated_globals;
            for bulk in next_input_frame.bulk {
                if let BulkAttribute::Particles(attribute) = bulk.meta.captured_attribute {
                    apply_particle_bulk(
                        &mut self.particles,
                        self.frame_input.header(),
                        attribute,
                        &bulk,
                    )
                    .map_err(|error| self.frame_input.next_frame_error(error))?;
                }
            }
        }

        self.frame_input
            .load_next()
            .map_err(SolverError::LoadNextInput)?;

        Ok((self.to_io_state(store_bvh, store_grid)?, Ok(())))
    }

    pub fn last_frame_substeps(&self) -> u32 {
        self.last_frame_substeps
    }
}

fn sorted_indices<'a>(
    reverse_sort_map: &'a [u32],
    header: &Header,
    bulk: &OwnedFrameBulk,
) -> Result<impl Iterator<Item = usize> + 'a, FrameVerificationError> {
    let ObjectParticles { particle_range, .. } =
        header.get_particle_object(&bulk.meta.object_name)?;
    Ok(reverse_sort_map[particle_range.clone()]
        .iter()
        .map(|index| *index as usize))
}

fn apply_particle_bulk(
    particles: &mut Particles,
    header: &Header,
    attribute: FrameBulkParticles,
    bulk: &OwnedFrameBulk,
) -> Result<(), FrameVerificationError> {
    let inv_scale = 1. / header.consts.simulation_scale;
    let indices = sorted_indices(&particles.reverse_sort_map, header, bulk)?;
    let parameters = &mut particles.parameters;
    match attribute {
        FrameBulkParticles::Flags => overwrite(
            &mut particles.flags,
            indices,
            bulk.assume_ints::<[u32; 2]>()?,
            |flags, [mask, values]| *flags = flags.overwritten(mask, values),
        ),
        FrameBulkParticles::ColliderBits => overwrite(
            &mut particles.collider_bits,
            indices,
            bulk.assume_ints::<u32>()?,
            |bits, value| *bits = value,
        ),
        FrameBulkParticles::Transform => {
            let transforms = bulk.assume_floats::<[[f32; 4]; 4]>()?;
            for (index, transform) in indices.zip(transforms) {
                let transform = Matrix4::from(*transform);
                particles.positions[index] = transform.fixed_view::<3, 1>(0, 3) * inv_scale;
                particles.position_gradients[index] = transform.fixed_view::<3, 3>(0, 0).into();
            }
        }
        FrameBulkParticles::Size => overwrite_floats(parameters, indices, bulk, |p, v| {
            p.initial_size = v * inv_scale
        })?,
        FrameBulkParticles::Density => {
            overwrite_floats(parameters, indices, bulk, |p, v| p.density = v)?
        }
        FrameBulkParticles::YoungsModulus => {
            overwrite_floats(parameters, indices, bulk, |p, v| p.youngs_modulus = v)?
        }
        FrameBulkParticles::PoissonsRatio => {
            overwrite_floats(parameters, indices, bulk, |p, v| p.poissons_ratio = v)?
        }
        FrameBulkParticles::ViscosityDynamic => {
            overwrite_floats(parameters, indices, bulk, |p, v| p.viscosity_dynamic = v)?
        }
        FrameBulkParticles::ViscosityBulk => {
            overwrite_floats(parameters, indices, bulk, |p, v| p.viscosity_bulk = v)?
        }
        FrameBulkParticles::BulkModulus => {
            overwrite_floats(parameters, indices, bulk, |p, v| p.bulk_modulus = v)?
        }
        FrameBulkParticles::SandAlpha => {
            overwrite_floats(parameters, indices, bulk, |p, v| p.sand_alpha = v)?
        }
        FrameBulkParticles::Exponent => overwrite(
            parameters,
            indices,
            bulk.assume_ints::<i32>()?,
            |p, value| p.exponent = value,
        ),
        FrameBulkParticles::InitialPosition => {
            overwrite_vectors(&mut particles.initial_positions, indices, bulk, inv_scale)?
        }
        FrameBulkParticles::Velocity => {
            overwrite_vectors(&mut particles.velocities, indices, bulk, inv_scale)?
        }
        FrameBulkParticles::GoalPosition => {
            overwrite_vectors(&mut particles.goal_positions, indices, bulk, inv_scale)?
        }
    }
    Ok(())
}

fn overwrite<T, V: Copy>(
    targets: &mut [T],
    indices: impl Iterator<Item = usize>,
    values: &[V],
    set: impl Fn(&mut T, V),
) {
    for (index, value) in indices.zip(values) {
        set(&mut targets[index], *value);
    }
}

fn overwrite_floats(
    parameters: &mut [ParticleParameters],
    indices: impl Iterator<Item = usize>,
    bulk: &OwnedFrameBulk,
    set: impl Fn(&mut ParticleParameters, f32),
) -> Result<(), FrameVerificationError> {
    overwrite(parameters, indices, bulk.assume_floats::<f32>()?, set);
    Ok(())
}

fn overwrite_vectors(
    vectors: &mut [Vector3<f32>],
    indices: impl Iterator<Item = usize>,
    bulk: &OwnedFrameBulk,
    inv_scale: f32,
) -> Result<(), FrameVerificationError> {
    overwrite(
        vectors,
        indices,
        bulk.assume_floats::<[f32; 3]>()?,
        |vector, value| *vector = Vector3::from(value) * inv_scale,
    );
    Ok(())
}
