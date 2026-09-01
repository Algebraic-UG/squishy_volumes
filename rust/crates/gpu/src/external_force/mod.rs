// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use nalgebra::Vector4;
use squishy_volumes_file_frame::ParticleFlags;
use std::num::NonZeroU32;

use super::*;

pub struct ExternalForce {
    external_force: CompiledModule,

    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub frames_per_second: u32,
}

pub struct Parameters;

pub struct Input {
    pub time: Allocation,
    pub time_step: Allocation,
    pub gravity: Allocation,
    pub particle_flags: Allocation,
    pub particle_positions_and_collider_bits: Allocation,
    pub particle_velocities: Allocation,
    pub particle_goals_start: Allocation,
    pub particle_goals_end: Allocation,
}

pub struct InputData<'a> {
    pub time: f32,
    pub time_step: f32,
    pub gravity: Vector4<f32>,
    pub particle_flags: &'a [ParticleFlags],
    pub particle_positions_and_collider_bits: &'a [PositionAndColliderBits],
    pub particle_velocities: &'a [Vector4<f32>],
    pub particle_goals_start: &'a [Vector4<f32>],
    pub particle_goals_end: &'a [Vector4<f32>],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        InputData {
            time,
            time_step,
            gravity,
            particle_flags,
            particle_positions_and_collider_bits,
            particle_velocities,
            particle_goals_start,
            particle_goals_end,
        }: InputData,
    ) -> Result<Self, GpuError> {
        let time = Allocation::new(device, "time", &[time])?;
        let time_step = Allocation::new(device, "time_step", &[time_step])?;
        let gravity = Allocation::new(device, "gravity", &[gravity])?;
        let particle_flags = Allocation::new(device, "particle_flags", particle_flags)?;
        let particle_positions_and_collider_bits = Allocation::new(
            device,
            "particle_positions_and_collider_bits",
            particle_positions_and_collider_bits,
        )?;
        let particle_velocities =
            Allocation::new(device, "particle_velocities", particle_velocities)?;
        let particle_goals_start =
            Allocation::new(device, "particle_goals_start", particle_goals_start)?;
        let particle_goals_end = Allocation::new(device, "particle_goals_end", particle_goals_end)?;

        Ok(Self {
            time,
            time_step,
            gravity,
            particle_flags,
            particle_positions_and_collider_bits,
            particle_velocities,
            particle_goals_start,
            particle_goals_end,
        })
    }
}

pub struct Output;

impl PipelinePart for ExternalForce {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            frames_per_second,
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            external_force,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (f32::MIN_BINDING_SIZE, false),                     // time
                    (f32::MIN_BINDING_SIZE, false),                     // time_step
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),          // gravity
                    (ParticleFlags::MIN_BINDING_SIZE, false),           // particle_flags
                    (PositionAndColliderBits::MIN_BINDING_SIZE, false), // particle_positions_and_collider_bits
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),          // particle_velocities
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),          // particle_goals_start
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),          // particle_goals_end
                ],
                immediate_size: 0,
                constants: [("FRAMES_PER_SECOND", frames_per_second as f64)]
            }
        );

        Ok(Self {
            external_force,
            workgroup_size,
            dispatch_limit,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            time,
            time_step,
            gravity,
            particle_flags,
            particle_positions_and_collider_bits,
            particle_velocities,
            particle_goals_start,
            particle_goals_end,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: particle_velocities.len::<Vector4<f32>>().get() as u32,
        })
        .direct();

        context
            .enter_module(
                encoder,
                &self.external_force,
                [
                    time.binding(),
                    time_step.binding(),
                    gravity.binding(),
                    particle_flags.binding(),
                    particle_positions_and_collider_bits.binding(),
                    particle_velocities.binding(),
                    particle_goals_start.binding(),
                    particle_goals_end.binding(),
                ],
            )
            .dispatch_workgroups(x, y, z);

        Ok(Output)
    }
}
