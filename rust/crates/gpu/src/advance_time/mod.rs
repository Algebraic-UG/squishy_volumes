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
