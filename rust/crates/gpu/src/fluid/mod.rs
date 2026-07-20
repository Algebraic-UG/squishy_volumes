// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use std::num::NonZeroU32;

use nalgebra::Matrix4x3;
use squishy_volumes_file_frame::{ParticleFlags, ParticleParameters};

use crate::particle_parameters::ParticleParametersDevice;

use super::*;

pub struct Fluid {
    fluid: CompiledModule,
    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub particle_flags: Allocation,
    pub particle_parameters: Allocation,
    pub particle_position_gradients: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        particle_flags: &[ParticleFlags],
        particle_parameters: &[ParticleParameters],
        particle_position_gradients: &[Matrix4x3<f32>],
    ) -> Result<Self, GpuError> {
        check_length!(particle_flags, particle_parameters)?;
        check_length!(particle_flags, particle_position_gradients)?;
        let particle_parameters = particle_parameters
            .iter()
            .map(Into::into)
            .collect::<Vec<ParticleParametersDevice>>();
        let particle_flags = Allocation::new(device, "particle_flags", particle_flags)?;
        let particle_parameters =
            Allocation::new(device, "particle_parameters", &particle_parameters)?;
        let particle_position_gradients = Allocation::new(
            device,
            "particle_position_gradients",
            particle_position_gradients,
        )?;

        Ok(Self {
            particle_flags,
            particle_parameters,
            particle_position_gradients,
        })
    }
}

pub struct Output;

impl PipelinePart for Fluid {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            fluid,
            CompiledModuleSettings {
                context,
                bind_group_entries: [
                    (ParticleFlags::MIN_BINDING_SIZE, false),
                    (ParticleParametersDevice::MIN_BINDING_SIZE, false),
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64)]
            }
        );

        Ok(Self {
            fluid,
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
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        assert_eq!(
            particle_flags.len::<ParticleFlags>(),
            particle_parameters.len::<ParticleParametersDevice>(),
        );
        assert_eq!(
            particle_flags.len::<ParticleFlags>(),
            particle_position_gradients.len::<Matrix4x3<f32>>()
        );
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: particle_parameters.len::<ParticleParametersDevice>().get() as u32,
        })
        .direct();

        context
            .enter_module(
                encoder,
                &self.fluid,
                [
                    particle_flags.binding(),
                    particle_parameters.binding(),
                    particle_position_gradients.binding(),
                ],
            )
            .dispatch_workgroups(x, y, z);

        Ok(Output)
    }
}
