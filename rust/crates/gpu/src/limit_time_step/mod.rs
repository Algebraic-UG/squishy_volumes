// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU32;

#[cfg(test)]
mod test;

use nalgebra::{Matrix4x3, Vector4};
use squishy_volumes_file_frame::ParticleFlags;
use squishy_volumes_util::ParticleParameters;

use crate::{particle_parameters::ParticleParametersDevice, time_step_limits::TimeStepLimits};

use super::*;

pub struct LimitTimeStep {
    limit_time_step_per_particle: LimitTimeStepPerParticle,
    find_minimum_time_step_limits: FindMinimumTimeStepLimits,
    combine_time_step_limits: CompiledModule,

    time_step_history_length: u32,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub grid_node_size: f32,
    pub max_time_step: f32,
    pub time_step_history_length: u32,
}

pub struct Parameters {
    pub current_step: u32,
}

pub struct Input {
    pub indirect_particles: Allocation,
    pub particle_flags: Allocation,
    pub particle_parameters: Allocation,
    pub particle_position_gradients: Allocation,
    pub particle_velocities: Allocation,
    pub particle_velocity_gradients: Allocation,
    pub limits_over_time: Allocation,
}

#[derive(Clone)]
pub struct InputData<'a> {
    pub particle_flags: &'a [ParticleFlags],
    pub particle_parameters: &'a [ParticleParameters],
    pub particle_position_gradients: &'a [Matrix4x3<f32>],
    pub particle_velocities: &'a [Vector4<f32>],
    pub particle_velocity_gradients: &'a [Matrix4x3<f32>],
    pub limits_over_time: &'a [TimeStepLimits],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
            ..
        }: Settings,
        InputData {
            particle_flags,
            particle_parameters,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
            limits_over_time,
        }: InputData,
    ) -> Result<Self, GpuError> {
        check_length!(particle_flags, particle_parameters)?;
        check_length!(particle_flags, particle_position_gradients)?;
        check_length!(particle_flags, particle_velocities)?;
        check_length!(particle_flags, particle_velocity_gradients)?;

        let indirect_particles = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: particle_flags.len() as u32,
        });

        let particle_parameters: Vec<ParticleParametersDevice> =
            particle_parameters.iter().map(Into::into).collect();

        let indirect_particles =
            Allocation::new(device, "indirect_particles", &[indirect_particles])?;
        let particle_flags = Allocation::new(device, "particle_parameters", particle_flags)?;
        let particle_parameters =
            Allocation::new(device, "particle_parameters", &particle_parameters)?;
        let particle_position_gradients = Allocation::new(
            device,
            "particle_position_gradients",
            particle_position_gradients,
        )?;
        let particle_velocities =
            Allocation::new(device, "particle_velocities", particle_velocities)?;
        let particle_velocity_gradients = Allocation::new(
            device,
            "particle_velocity_gradients",
            particle_velocity_gradients,
        )?;
        let limits_over_time = Allocation::new(device, "limits_over_time", limits_over_time)?;

        Ok(Self {
            indirect_particles,
            particle_flags,
            particle_parameters,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
            limits_over_time,
        })
    }
}

pub struct Output {
    pub time_step: Allocation,
}

impl PipelinePart for LimitTimeStep {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            grid_node_size,
            max_time_step,
            time_step_history_length,
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let limit_time_step_per_particle = LimitTimeStepPerParticle::new(
            context,
            limit_time_step_per_particle::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size,
            },
        )?;

        let find_minimum_time_step_limits = FindMinimumTimeStepLimits::new(
            context,
            find_minimum_time_step_limits::Settings {
                workgroup_size,
                dispatch_limit,
            },
        )?;

        let_compiled_module!(
            combine_time_step_limits,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (TimeStepLimits::MIN_BINDING_SIZE, false),
                    (f32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("MAX_TIME_STEP", max_time_step as f64)]
            }
        );

        if combine_time_step_limits.subgroup_size.get() < time_step_history_length {
            return Err(GpuPipelineCreationError::SubgroupSizeTooSmall {
                label: combine_time_step_limits.label.unwrap(),
                subgroup_size: combine_time_step_limits.subgroup_size.get(),
                needed: time_step_history_length,
            });
        }

        Ok(Self {
            limit_time_step_per_particle,
            find_minimum_time_step_limits,
            combine_time_step_limits,
            time_step_history_length,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_particles,
            particle_flags,
            particle_parameters,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
            limits_over_time,
        }: Input,
        Parameters { current_step }: Parameters,
    ) -> Result<Output, GpuError> {
        assert!((current_step as u64) < limits_over_time.len::<TimeStepLimits>().get());

        let limit_time_step_per_particle::Output { time_step_limits } =
            self.limit_time_step_per_particle.record(
                context,
                encoder,
                limit_time_step_per_particle::Input {
                    particle_flags,
                    particle_parameters,
                    particle_position_gradients,
                    particle_velocities,
                    particle_velocity_gradients,
                },
                limit_time_step_per_particle::Parameters,
            )?;

        let find_minimum_time_step_limits::Output {
            minimum_time_step_limits,
        } = self.find_minimum_time_step_limits.record(
            context,
            encoder,
            find_minimum_time_step_limits::Input {
                indirect: indirect_particles,
                time_step_limits,
            },
            find_minimum_time_step_limits::Parameters,
        )?;

        encoder
            .scope(Some("copy_time_step_mininum"))
            .copy_buffer_to_buffer(
                minimum_time_step_limits.buffer(),
                minimum_time_step_limits.offset(),
                limits_over_time.buffer(),
                limits_over_time.offset()
                    + TimeStepLimits::MIN_BINDING_SIZE.get() * current_step as u64,
                Some(TimeStepLimits::MIN_BINDING_SIZE.get()),
            );

        let limits_over_time_binding = {
            let valid_entries = current_step + 1;
            let size = valid_entries.min(self.time_step_history_length);
            let offset = valid_entries - size;
            tracing::warn!(current_step, valid_entries, size, offset);
            wgpu::BufferBinding {
                buffer: limits_over_time.buffer(),
                offset: offset as u64 * TimeStepLimits::MIN_BINDING_SIZE.get(),
                size: Some(
                    (size as u64 * TimeStepLimits::MIN_BINDING_SIZE.get())
                        .try_into()
                        .unwrap(),
                ),
            }
        };

        let time_step = context
            .allocator()?
            .allocate::<f32>("time_step", 1.try_into().unwrap())?;

        context
            .enter_module(
                encoder,
                &self.combine_time_step_limits,
                [limits_over_time_binding, time_step.binding()],
            )
            .dispatch_workgroups(self.combine_time_step_limits.subgroup_size.get(), 1, 1);

        Ok(Output { time_step })
    }
}
