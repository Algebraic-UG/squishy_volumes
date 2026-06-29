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

use super::*;

pub struct Sand {
    sand: CompiledModule,
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
    pub particle_parameters: Allocation,
    pub particle_position_gradients: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        particle_parameters: &[particle_parameters::Device],
        particle_position_gradients: &[Matrix4x3<f32>],
    ) -> Result<Self, GpuError> {
        check_length!(particle_parameters, particle_position_gradients)?;
        let particle_parameters =
            Allocation::new(device, "particle_parameters", particle_parameters)?;
        let particle_position_gradients = Allocation::new(
            device,
            "particle_position_gradients",
            particle_position_gradients,
        )?;

        Ok(Self {
            particle_parameters,
            particle_position_gradients,
        })
    }
}

pub struct Output;

impl PipelinePart for Sand {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
    ) -> Self {
        let_compiled_module!(
            sand,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (particle_parameters::Device::MIN_BINDING_SIZE, false),
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64)]
            }
        );

        Self {
            sand,
            workgroup_size,
            dispatch_limit,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            particle_parameters,
            particle_position_gradients,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        assert_eq!(
            particle_parameters.len::<particle_parameters::Device>(),
            particle_position_gradients.len::<Matrix4x3<f32>>()
        );
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: particle_parameters
                .len::<particle_parameters::Device>()
                .get() as u32,
        })
        .direct();

        let mut compute_pass = encoder.begin_compute_pass(self.sand.label);
        compute_pass.set_pipeline(&self.sand.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.sand,
                [
                    particle_parameters.binding(),
                    particle_position_gradients.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups(x, y, z);

        Ok(Output)
    }
}
