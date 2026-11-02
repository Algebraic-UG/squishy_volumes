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

use super::*;

pub struct BitsToPops {
    bits_to_pops: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub bits: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings { workgroup_size }: Settings,
        dispatch_limit: NonZeroU32,
        bits: &[u32],
    ) -> Result<Self, GpuAllocatorError> {
        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: bits.len() as u32,
        });

        let indirect = Allocation::new(device, "indirect", &[indirect])?;
        let bits = Allocation::new(device, "bits", bits)?;

        Ok(Self { indirect, bits })
    }
}

pub struct Output {
    pub pops: Allocation,
}

impl PipelinePart for BitsToPops {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings { workgroup_size }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            bits_to_pops,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: []
            }
        );

        Ok(Self { bits_to_pops })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input { indirect, bits }: Input,
        _: Self::Parameters,
    ) -> Result<Output, GpuError> {
        let pops = context
            .allocator()?
            .allocate::<u32>("pops", bits.len::<u32>())?;

        context
            .enter_module(
                encoder,
                &self.bits_to_pops,
                [indirect.binding(), bits.binding(), pops.binding()],
            )
            .dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());

        Ok(Output { pops })
    }
}
