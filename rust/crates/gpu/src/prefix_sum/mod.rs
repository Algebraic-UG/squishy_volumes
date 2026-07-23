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

impl PrefixSum {
    pub fn representative_module(&self) -> &CompiledModule {
        &self.prepare_indirect
    }
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
    ) -> Result<Self, GpuAllocatorError> {
        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: numbers.len() as u32,
        });

        let numbers = Allocation::new(device, "numbers", numbers)?;
        let indirect = Allocation::new(device, "indirect", &[indirect])?;

        Ok(Self { indirect, numbers })
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

    fn new(
        context: &mut GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            prepare_indirect,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, false),
                    (Indirect::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("DISPATCH_LIMIT", dispatch_limit.get() as f64),],
            }
        );

        let_compiled_module!(
            build_levels,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 4,
                constants: [],
            }
        );

        let_compiled_module!(
            fill_final,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false)
                ],
                immediate_size: 0,
                constants: [],
            }
        );

        let subgroup_size = prepare_indirect.subgroup_size.get();

        let_compiled_module!(
            total_sum,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false)
                ],
                immediate_size: 0,
                constants: [],
            }
        );

        prepare_indirect.check_same_sugroup_size(&build_levels)?;
        prepare_indirect.check_same_sugroup_size(&fill_final)?;
        prepare_indirect.check_same_sugroup_size(&total_sum)?;
        prepare_indirect.check_workgroup_size_multiple_of_subgroup_size(workgroup_size.get())?;

        Ok(Self {
            subgroup_size,
            prepare_indirect,
            build_levels,
            fill_final,
            total_sum,
        })
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

        context
            .enter_module(
                encoder,
                &self.prepare_indirect,
                [input.indirect.binding(), indirect_levels.binding()],
            )
            .dispatch_workgroups(1, 1, 1);

        let mut compute_pass = context.enter_module(
            encoder,
            &self.build_levels,
            [input.indirect.binding(), input.numbers.binding()],
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

        context
            .enter_module(
                encoder,
                &self.fill_final,
                [
                    input.indirect.binding(),
                    input.numbers.binding(),
                    prefix_sums.binding(),
                ],
            )
            .dispatch_workgroups_indirect(input.indirect.buffer(), input.indirect.offset());

        let total_sum = if total_sum {
            let total_sum = context
                .allocator()?
                .allocate::<u32>("total_sum", 1.try_into().unwrap())?;
            context
                .enter_module(
                    encoder,
                    &self.total_sum,
                    [
                        input.indirect.binding(),
                        input.numbers.binding(),
                        total_sum.binding(),
                    ],
                )
                .dispatch_workgroups(1, 1, 1);
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
