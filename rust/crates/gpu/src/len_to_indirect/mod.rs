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

pub struct LenToIndirect {
    len_to_indirect: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub len: Allocation,
}

impl Input {
    pub fn new(device: &wgpu::Device, len: u32) -> Self {
        let len = Allocation::new(device, "len", &[len]);
        Self { len }
    }
}

pub struct Output {
    pub new_indirect: Allocation,
}

impl PipelinePart for LenToIndirect {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
    ) -> Self {
        let device = context.device();
        let_compiled_module!(
            len_to_indirect,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (u32::MIN_BINDING_SIZE, true),
                    (Indirect::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("DISPATCH_LIMIT", dispatch_limit.get() as f64),
                ],
            }
        );

        Self { len_to_indirect }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input { len }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let new_indirect = context
            .indirect_allocator()?
            .allocate::<Indirect>("new_indirect", 1.try_into().unwrap())?;

        let mut compute_pass = encoder.begin_compute_pass(self.len_to_indirect.label);
        compute_pass.set_pipeline(&self.len_to_indirect.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.len_to_indirect,
                [len.binding(), new_indirect.binding()],
            ),
            &[],
        );

        compute_pass.dispatch_workgroups(1, 1, 1);

        Ok(Output { new_indirect })
    }
}
