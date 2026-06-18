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

pub struct CountsIndirect {
    counts_indirect: CompiledModule,
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
    ) -> Self {
        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len,
        });
        let indirect = Allocation::new(device, "indirect", &[indirect]);
        Self { indirect }
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
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            bit_count,
        }: Settings,
    ) -> Self {
        let device = context.device();
        let_compiled_module!(
            counts_indirect,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, false),
                    (Indirect::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("DISPATCH_LIMIT", dispatch_limit.get() as f64),
                    ("BIT_COUNT", bit_count.get() as f64),
                ],
            }
        );

        Self { counts_indirect }
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

        let mut compute_pass = encoder.begin_compute_pass(self.counts_indirect.label);
        compute_pass.set_pipeline(&self.counts_indirect.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.counts_indirect,
                [indirect.binding(), indirect_counts.binding()],
            ),
            &[],
        );

        compute_pass.dispatch_workgroups(1, 1, 1);

        Ok(Output { indirect_counts })
    }
}
