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

use std::num::NonZeroU32;

#[cfg(test)]
mod test;

use nalgebra::{Matrix4x3, Vector4};
use squishy_volumes_util::{ParticleFlags, ParticleParameters};

use crate::time_step_limits::TimeStepLimits;

use super::*;

pub struct LimitTimeStepPerParticle {
    limit_time_step_per_particle: CompiledModule,
    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub grid_node_size: f32,
}

pub struct Parameters;

pub struct Input {
    pub particle_flags: Allocation,
    pub particle_parameters: Allocation,
    pub particle_position_gradients: Allocation,
    pub particle_velocities: Allocation,
    pub particle_velocity_gradients: Allocation,
}

#[derive(Clone)]
pub struct InputData<'a> {
    pub particle_flags: &'a [ParticleFlags],
    pub particle_parameters: &'a [ParticleParameters],
    pub particle_position_gradients: &'a [Matrix4x3<f32>],
    pub particle_velocities: &'a [Vector4<f32>],
    pub particle_velocity_gradients: &'a [Matrix4x3<f32>],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        InputData {
            particle_flags,
            particle_parameters,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        }: InputData,
    ) -> Result<Self, GpuError> {
        check_length!(particle_flags, particle_parameters)?;
        check_length!(particle_flags, particle_position_gradients)?;
        check_length!(particle_flags, particle_velocities)?;
        check_length!(particle_flags, particle_velocity_gradients)?;

        let particle_flags = Allocation::new(device, "particle_parameters", particle_flags)?;
        let particle_parameters =
            Allocation::new(device, "particle_parameters", particle_parameters)?;
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

        Ok(Self {
            particle_flags,
            particle_parameters,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        })
    }
}

pub struct Output {
    pub time_step_limits: Allocation,
}

impl PipelinePart for LimitTimeStepPerParticle {
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
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            limit_time_step_per_particle,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (ParticleFlags::MIN_BINDING_SIZE, false),      // flags
                    (ParticleParameters::MIN_BINDING_SIZE, false), // parameters
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),   // position_gradients
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),     // velocities
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),   // velocity_gradients
                    (TimeStepLimits::MIN_BINDING_SIZE, false),     // time_step_limits
                ],
                immediate_size: 0,
                constants: [("GRID_NODE_SIZE", grid_node_size as f64),]
            }
        );

        Ok(Self {
            limit_time_step_per_particle,
            workgroup_size,
            dispatch_limit,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            particle_flags,
            particle_parameters,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let num_particles = particle_flags.len::<ParticleFlags>();
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: num_particles.get() as u32,
        })
        .direct();

        let time_step_limits = context
            .allocator()?
            .allocate::<TimeStepLimits>("time_step_limits", num_particles)?;

        context
            .enter_module(
                encoder,
                &self.limit_time_step_per_particle,
                [
                    particle_flags.binding(),
                    particle_parameters.binding(),
                    particle_position_gradients.binding(),
                    particle_velocities.binding(),
                    particle_velocity_gradients.binding(),
                    time_step_limits.binding(),
                ],
            )
            .dispatch_workgroups(x, y, z);

        Ok(Output { time_step_limits })
    }
}
