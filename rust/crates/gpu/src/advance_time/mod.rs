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

pub struct AdvanceTime {
    advance_time: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub frames_per_second: u32,
}

pub struct Parameters;

pub struct Input {
    pub time_step: Allocation,
    pub time: Allocation,
    pub step: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        time_step: f32,
        time: f32,
        step: u32,
    ) -> Result<Self, GpuAllocatorError> {
        let time_step = Allocation::new(device, "time_step", &[time_step])?;
        let time = Allocation::new(device, "time", &[time])?;
        let step = Allocation::new(device, "step", &[step])?;
        Ok(Self {
            time_step,
            time,
            step,
        })
    }
}

pub struct Output;

impl PipelinePart for AdvanceTime {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings {
            workgroup_size,
            frames_per_second,
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            advance_time,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (f32::MIN_BINDING_SIZE, false),
                    (f32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("FRAMES_PER_SECOND", frames_per_second as f64)],
            }
        );

        Ok(Self { advance_time })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            time_step,
            time,
            step,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        context
            .enter_module(
                encoder,
                &self.advance_time,
                [time_step.binding(), time.binding(), step.binding()],
            )
            .dispatch_workgroups(1, 1, 1);

        Ok(Output)
    }
}
