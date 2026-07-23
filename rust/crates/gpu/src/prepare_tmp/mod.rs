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

use nalgebra::{Matrix4, Matrix4x3, Vector4};
use squishy_volumes_file_frame::{ParticleFlags, ParticleParameters};

use crate::particle_parameters::ParticleParametersDevice;

use super::*;

pub struct PrepareTmp {
    prepare_tmp: CompiledModule,

    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub grid_node_size: f32,
    pub time_step: f32,
}

pub struct Parameters;

pub struct Input {
    pub particle_flags: Allocation,
    pub particle_parameters: Allocation,
    pub particle_positions_and_collider_bits: Allocation,
    pub particle_position_gradients: Allocation,
    pub particle_velocities: Allocation,
    pub particle_velocity_gradients: Allocation,
}

#[derive(Clone)]
pub struct InputData<'a> {
    pub particle_flags: &'a [ParticleFlags],
    pub particle_parameters: &'a [ParticleParameters],
    pub particle_positions_and_collider_bits: &'a [PositionAndColliderBits],
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
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        }: InputData,
    ) -> Result<Self, GpuError> {
        check_length!(particle_flags, particle_parameters)?;
        check_length!(particle_flags, particle_positions_and_collider_bits)?;
        check_length!(particle_flags, particle_position_gradients)?;
        check_length!(particle_flags, particle_velocities)?;
        check_length!(particle_flags, particle_velocity_gradients)?;

        let particle_parameters: Vec<ParticleParametersDevice> =
            particle_parameters.iter().map(Into::into).collect();

        let particle_flags = Allocation::new(device, "particle_parameters", particle_flags)?;
        let particle_parameters =
            Allocation::new(device, "particle_parameters", &particle_parameters)?;
        let particle_positions_and_collider_bits = Allocation::new(
            device,
            "particle_positions_and_collider_bits",
            particle_positions_and_collider_bits,
        )?;
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
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        })
    }
}

pub struct Output {
    pub particle_tmp: Allocation,
}

impl PipelinePart for PrepareTmp {
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
            time_step,
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            prepare_tmp,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (ParticleFlags::MIN_BINDING_SIZE, false),            // flags
                    (ParticleParametersDevice::MIN_BINDING_SIZE, false), // parameters
                    (PositionAndColliderBits::MIN_BINDING_SIZE, false), // particle_positions_and_collider_bits
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),        // position_gradients
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),          // velocities
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),        // velocity_gradients
                    (Matrix4::<f32>::MIN_BINDING_SIZE, false),          // tmp
                ],
                immediate_size: 0,
                constants: [
                    ("GRID_NODE_SIZE", grid_node_size as f64),
                    ("TIME_STEP", time_step as f64),
                ]
            }
        );

        Ok(Self {
            prepare_tmp,
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
            particle_positions_and_collider_bits,
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

        let particle_tmp = context
            .allocator()?
            .allocate::<Matrix4<f32>>("particle_tmp", num_particles)?;

        context
            .enter_module(
                encoder,
                &self.prepare_tmp,
                [
                    particle_flags.binding(),
                    particle_parameters.binding(),
                    particle_positions_and_collider_bits.binding(),
                    particle_position_gradients.binding(),
                    particle_velocities.binding(),
                    particle_velocity_gradients.binding(),
                    particle_tmp.binding(),
                ],
            )
            .dispatch_workgroups(x, y, z);

        Ok(Output { particle_tmp })
    }
}
