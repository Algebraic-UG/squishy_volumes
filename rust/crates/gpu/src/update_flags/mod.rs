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

#[cfg(test)]
mod test;

use std::num::NonZeroU32;

use squishy_volumes_util::ParticleFlags;

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

pub struct Parameters {
    pub offset: u32,
}

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
        check_range!(new_flags, flags, 0)?;

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
                immediate_size: 4,
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
        Parameters { offset }: Parameters,
    ) -> Result<Output, GpuError> {
        let num_flags = flags.len::<ParticleFlags>();
        let num_new_flags = new_flags.len::<ParticleFlags>();
        if num_new_flags.get() + offset as u64 > num_flags.get() {
            Err(GpuInputError::RangesDoesNotFit {
                a: "new_flags",
                a_len: num_new_flags.get() as usize,
                b: "flags",
                b_len: num_flags.get() as usize,
                offset: offset as usize,
            })?;
        }
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: num_flags.get() as u32,
        })
        .direct();

        let mut compute_pass = context.enter_module(
            encoder,
            &self.update_flags,
            [new_flags.binding(), flags.binding()],
        );
        compute_pass.set_immediates(0, bytemuck::bytes_of(&offset));
        compute_pass.dispatch_workgroups(x, y, z);

        Ok(Output)
    }
}
