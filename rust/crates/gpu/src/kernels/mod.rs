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

pub struct Kernels {
    workgroup_size: NonZeroU32,
    kernels: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    values: Allocation,
}

impl Input {
    pub fn new(device: &wgpu::Device, values: &[f32]) -> Result<Self, GpuAllocatorError> {
        let values = Allocation::new(device, "values", values)?;

        Ok(Self { values })
    }
}

pub struct Output {
    pub linear: Allocation,
    pub quadratic: Allocation,
    pub cubic: Allocation,
}

impl PipelinePart for Kernels {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings { workgroup_size }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            kernels,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (f32::MIN_BINDING_SIZE, false),
                    (f32::MIN_BINDING_SIZE, false),
                    (f32::MIN_BINDING_SIZE, false),
                    (f32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [],
            }
        );

        Ok(Self {
            workgroup_size,
            kernels,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input { values }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let linear = context
            .allocator()?
            .allocate::<f32>("linear", values.len::<f32>())?;
        let quadratic = context
            .allocator()?
            .allocate::<f32>("quadratic", values.len::<f32>())?;
        let cubic = context
            .allocator()?
            .allocate::<f32>("cubic", values.len::<f32>())?;

        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            len: values.len::<f32>().get() as u32,
        })
        .direct();

        context
            .enter_module(
                encoder,
                &self.kernels,
                [
                    values.binding(),
                    linear.binding(),
                    quadratic.binding(),
                    cubic.binding(),
                ],
            )
            .dispatch_workgroups(x, y, z);

        Ok(Output {
            linear,
            quadratic,
            cubic,
        })
    }
}
