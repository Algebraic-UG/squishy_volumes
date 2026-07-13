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

    fn new(context: &mut GpuContext, Settings { workgroup_size }: Settings) -> Self {
        let_compiled_module!(
            kernels,
            CompiledModuleSettings {
                context,
                bind_group_entries: [
                    (f32::MIN_BINDING_SIZE, false),
                    (f32::MIN_BINDING_SIZE, false),
                    (f32::MIN_BINDING_SIZE, false),
                    (f32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64),],
            }
        );

        Self {
            workgroup_size,
            kernels,
        }
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
