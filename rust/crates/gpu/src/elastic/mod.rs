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

pub struct Elastic {
    stress_and_energy: CompiledModule,
}

pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub position_gradients: Allocation,
    pub particle_flags: Allocation,
    pub particle_parameters: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        workgroup_size: NonZeroU32,
        dispatch_limit: NonZeroU32,
        position_gradients: &[Matrix4x3<f32>],
        particle_flags: &[particle_parameters::Flags],
        particle_parameters: &[particle_parameters::Device],
    ) -> Result<Self, GpuError> {
        check_length!(position_gradients, particle_parameters)?;
        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: position_gradients.len() as u32,
        });
        let indirect = Allocation::new(device, "indirect", &[indirect])?;
        let position_gradients = Allocation::new(device, "position_gradients", position_gradients)?;
        let particle_flags = Allocation::new(device, "particle_flags", particle_flags)?;
        let particle_parameters =
            Allocation::new(device, "particle_parameters", particle_parameters)?;

        Ok(Self {
            indirect,
            position_gradients,
            particle_flags,
            particle_parameters,
        })
    }
}

pub struct Output {
    pub stresses: Allocation,
    pub energies: Allocation,
}

impl PipelinePart for Elastic {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &mut GpuContext, Settings { workgroup_size }: Settings) -> Self {
        let_compiled_module!(
            stress_and_energy,
            CompiledModuleSettings {
                context,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),
                    (particle_parameters::Flags::MIN_BINDING_SIZE, false),
                    (particle_parameters::Device::MIN_BINDING_SIZE, false),
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),
                    (f32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64)]
            }
        );

        Self { stress_and_energy }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            position_gradients,
            particle_flags,
            particle_parameters,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let stresses = context
            .allocator()?
            .allocate::<Matrix4x3<f32>>("stresses", position_gradients.len::<Matrix4x3<f32>>())?;
        let energies = context
            .allocator()?
            .allocate::<f32>("energies", position_gradients.len::<Matrix4x3<f32>>())?;

        let mut compute_pass = context.enter_module(
            encoder,
            &self.stress_and_energy,
            [
                indirect.binding(),
                position_gradients.binding(),
                particle_flags.binding(),
                particle_parameters.binding(),
                stresses.binding(),
                energies.binding(),
            ],
        );
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());
        drop(compute_pass);

        Ok(Output { stresses, energies })
    }
}
