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

use squishy_volumes_file_frame::ParticleFlags;

use super::*;

pub struct UpdateFlags {
    update_flags: CompiledModule,
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
    pub new_flags: Allocation,
    pub flags: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        new_flags: &[ParticleFlags],
        flags: &[ParticleFlags],
    ) -> Result<Self, GpuError> {
        check_length!(new_flags, flags)?;

        let new_flags = Allocation::new(device, "new_flags", new_flags)?;
        let flags = Allocation::new(device, "flags", flags)?;

        Ok(Self { flags, new_flags })
    }
}

pub struct Output;

impl PipelinePart for UpdateFlags {
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
            update_flags,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (ParticleFlags::MIN_BINDING_SIZE, false),
                    (ParticleFlags::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: []
            }
        );

        Ok(Self {
            update_flags,
            workgroup_size,
            dispatch_limit,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input { new_flags, flags }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let num_flags = flags.len::<ParticleFlags>();
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: num_flags.get() as u32,
        })
        .direct();

        context
            .enter_module(
                encoder,
                &self.update_flags,
                [new_flags.binding(), flags.binding()],
            )
            .dispatch_workgroups(x, y, z);

        Ok(Output)
    }
}
