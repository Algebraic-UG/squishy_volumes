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

pub struct RecycleToIndirect {
    recycle_to_indirect: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub count_prefix_sums: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
        len: u32,
        count_prefix_sums: &[u32],
    ) -> Self {
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len,
        });
        let indirect = Allocation::new(device, "indirect", &[indirect]);
        let count_prefix_sums = Allocation::new(device, "count_prefix_sums", count_prefix_sums);
        Self {
            indirect,
            count_prefix_sums,
        }
    }
}

pub struct Output {
    pub indirect_colors: Allocation,
    pub indirect_colors_batch: Allocation,
}

impl PipelinePart for RecycleToIndirect {
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
            recycle_to_indirect,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, true),
                    (Indirect::MIN_BINDING_SIZE, false),
                    (Indirect::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("DISPATCH_LIMIT", dispatch_limit.get() as f64),
                ]
            }
        );

        Self {
            recycle_to_indirect,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            count_prefix_sums,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let indirect_colors = context
            .indirect_allocator()?
            .allocate::<Indirect>("indirect_colors", 8.try_into().unwrap())?;
        let indirect_colors_batch = context
            .indirect_allocator()?
            .allocate::<Indirect>("indirect_colors_batch", 8.try_into().unwrap())?;

        let mut compute_pass = encoder.begin_compute_pass(self.recycle_to_indirect.label);
        compute_pass.set_pipeline(&self.recycle_to_indirect.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.recycle_to_indirect,
                [
                    indirect.binding(),
                    count_prefix_sums.binding(),
                    indirect_colors.binding(),
                    indirect_colors_batch.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups(1, 1, 1);

        Ok(Output {
            indirect_colors,
            indirect_colors_batch,
        })
    }
}
