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

pub struct CountsIndirect {
    counts_indirect: CompiledModule,
}

impl CountsIndirect {
    pub fn representative_module(&self) -> &CompiledModule {
        &self.counts_indirect
    }
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub bit_count: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
            ..
        }: Settings,
        len: u32,
    ) -> Result<Self, GpuAllocatorError> {
        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len,
        });
        let indirect = Allocation::new(device, "indirect", &[indirect])?;
        Ok(Self { indirect })
    }
}

pub struct Output {
    pub indirect_counts: Allocation,
}

impl PipelinePart for CountsIndirect {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            bit_count,
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            counts_indirect,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, false),
                    (Indirect::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("DISPATCH_LIMIT", dispatch_limit.get() as f64),
                    ("DISPATCH_WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("BIT_COUNT", bit_count.get() as f64),
                ],
            }
        );

        Ok(Self { counts_indirect })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input { indirect }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let indirect_counts = context
            .indirect_allocator()?
            .allocate::<Indirect>("counts_indirect", 1.try_into().unwrap())?;

        context
            .enter_module(
                encoder,
                &self.counts_indirect,
                [indirect.binding(), indirect_counts.binding()],
            )
            .dispatch_workgroups(1, 1, 1);

        Ok(Output { indirect_counts })
    }
}
