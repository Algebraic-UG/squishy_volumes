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

pub struct FindMinimum {
    subgroup_size: u32,
    prepare_indirect: CompiledModule,
    build_levels_minimum: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

pub struct Parameters;

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
        numbers: &[f32],
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
    pub minimum: Allocation,
}

impl PipelinePart for FindMinimum {
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
            build_levels_minimum,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (f32::MIN_BINDING_SIZE, false),
                    (f32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 4,
                constants: [],
            }
        );

        let subgroup_size = prepare_indirect.subgroup_size.get();

        prepare_indirect.check_same_sugroup_size(&build_levels_minimum)?;
        prepare_indirect.check_workgroup_size_multiple_of_subgroup_size(workgroup_size.get())?;

        Ok(Self {
            subgroup_size,
            prepare_indirect,
            build_levels_minimum,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        input: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let len = input.numbers.len::<f32>();

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

        let minimum = context
            .allocator()?
            .allocate::<f32>("minumum", 1.try_into().unwrap())?;

        let mut compute_pass = context.enter_module(
            encoder,
            &self.build_levels_minimum,
            [
                input.indirect.binding(),
                input.numbers.binding(),
                minimum.binding(),
            ],
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

        Ok(Output { minimum })
    }
}
