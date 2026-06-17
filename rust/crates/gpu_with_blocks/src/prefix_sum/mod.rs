// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::{NonZeroU32, NonZeroU64};

use super::*;

#[cfg(test)]
mod test;

pub struct PrefixSum {
    subgroup_size: u32,
    prepare_indirect: CompiledModule,
    build_levels: CompiledModule,
    fill_final: CompiledModule,
    total_sum: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

pub struct Parameters {
    pub total_sum: bool,
}

pub struct Input {
    pub indirect: Allocation,
    pub numbers: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
        numbers: &[u32],
    ) -> Self {
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: numbers.len() as u32,
        });

        let numbers = Allocation::new(device, "numbers", numbers);
        let indirect = Allocation::new(device, "indirect", &[indirect]);

        Self { indirect, numbers }
    }
}

pub struct Output {
    pub prefix_sums: Allocation,
    pub total_sum: Option<Allocation>,
}

impl PipelinePart for PrefixSum {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &GpuContext, settings: Settings) -> Self {
        let workgroup_size = settings.workgroup_size.get();
        let dispatch_limit = settings.dispatch_limit.get();
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size.is_multiple_of(subgroup_size));

        let_compiled_module!(
            prepare_indirect,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, false),
                    (Indirect::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size as f64),
                    ("DISPATCH_LIMIT", dispatch_limit as f64),
                ],
            }
        );

        let_compiled_module!(
            build_levels,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 4,
                constants: [("WORKGROUP_SIZE", workgroup_size as f64)],
            }
        );

        let_compiled_module!(
            fill_final,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false)
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size as f64)],
            }
        );

        let_compiled_module!(
            total_sum,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false)
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", subgroup_size as f64)],
            }
        );

        Self {
            subgroup_size,
            prepare_indirect,
            build_levels,
            fill_final,
            total_sum,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        input: Input,
        Parameters { total_sum }: Parameters,
    ) -> Result<Output, GpuError> {
        let len = input.numbers.len::<u32>();

        let max_level = (len.get() as u32 * self.subgroup_size - 1).ilog(self.subgroup_size);

        let indirect_levels = context.indirect_allocator()?.allocate::<Indirect>(
            "indiret_levels",
            NonZeroU64::new(max_level as u64 + 1).unwrap(),
        )?;

        let mut compute_pass = encoder.begin_compute_pass(self.prepare_indirect.label);
        compute_pass.set_pipeline(&self.prepare_indirect.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.prepare_indirect,
                [input.indirect.binding(), indirect_levels.binding()],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups(1, 1, 1);
        drop(compute_pass);

        let mut compute_pass = encoder.begin_compute_pass(self.build_levels.label);
        compute_pass.set_pipeline(&self.build_levels.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.build_levels,
                [input.indirect.binding(), input.numbers.binding()],
            ),
            &[],
        );
        for level in 0..max_level {
            let stride = self.subgroup_size.pow(level);
            compute_pass.set_immediates(0, bytemuck::bytes_of(&stride));
            compute_pass.dispatch_workgroups_indirect(
                indirect_levels.buffer(),
                indirect_levels.offset() + level as u64 * Indirect::MIN_BINDING_SIZE.get(),
            );
        }
        drop(compute_pass);

        let prefix_sums = context.allocator()?.allocate::<u32>("prefix_sums", len)?;

        let mut compute_pass = encoder.begin_compute_pass(self.fill_final.label);
        compute_pass.set_pipeline(&self.fill_final.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.fill_final,
                [
                    input.indirect.binding(),
                    input.numbers.binding(),
                    prefix_sums.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(input.indirect.buffer(), input.indirect.offset());
        drop(compute_pass);

        let total_sum = if total_sum {
            let total_sum = context
                .allocator()?
                .allocate::<u32>("total_sum", 1.try_into().unwrap())?;
            let mut compute_pass = encoder.begin_compute_pass(self.total_sum.label);
            compute_pass.set_pipeline(&self.total_sum.compute_pipeline);
            compute_pass.set_bind_group(
                0,
                &create_bind_group(
                    context.device(),
                    &self.fill_final,
                    [
                        input.indirect.binding(),
                        input.numbers.binding(),
                        total_sum.binding(),
                    ],
                ),
                &[],
            );
            compute_pass.dispatch_workgroups(1, 1, 1);
            Some(total_sum)
        } else {
            None
        };

        Ok(Output {
            prefix_sums,
            total_sum,
        })
    }
}
