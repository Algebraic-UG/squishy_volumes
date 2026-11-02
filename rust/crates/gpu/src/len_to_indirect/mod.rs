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

use super::*;

pub struct LenToIndirect {
    len_to_indirect: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

pub struct Parameters {
    pub limit: u32,
}

pub struct Input {
    pub len: Allocation,
    pub indirect: Allocation,
}

impl Input {
    pub fn new(device: &wgpu::Device, len: u32) -> Result<Self, GpuAllocatorError> {
        let len = Allocation::new(device, "len", &[len])?;
        let indirect = Allocation::new(device, "indirect", &[Indirect::default()])?;
        Ok(Self { len, indirect })
    }
}

pub struct Output;

impl PipelinePart for LenToIndirect {
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
            len_to_indirect,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (u32::MIN_BINDING_SIZE, true),
                    (Indirect::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 4,
                constants: [
                    ("DISPATCH_WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("DISPATCH_LIMIT", dispatch_limit.get() as f64),
                ],
            }
        );

        Ok(Self { len_to_indirect })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input { len, indirect }: Input,
        Parameters { limit }: Parameters,
    ) -> Result<Output, GpuError> {
        let mut compute_pass = context.enter_module(
            encoder,
            &self.len_to_indirect,
            [len.binding(), indirect.binding()],
        );
        compute_pass.set_immediates(0, bytemuck::bytes_of(&limit));
        compute_pass.dispatch_workgroups(1, 1, 1);
        drop(compute_pass);

        Ok(Output)
    }
}
