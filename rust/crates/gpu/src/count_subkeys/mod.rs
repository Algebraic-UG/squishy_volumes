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

pub mod standalone;

pub struct CountSubkeys {
    workgroup_size: u32,
    dispatch_limit: u32,
    subgroup_size: u32,
    bit_count: u32,
    count_subkeys: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub bit_count: NonZeroU32,
}

pub struct Parameters {
    pub bit_offset: u32,
}

pub struct InputBindings {
    pub indirect: Allocation,
    pub indices: Allocation,
    pub keys: Allocation,
}

pub struct OutputBindings {
    pub counts: Allocation,
}

impl PipelinePart for CountSubkeys {
    type Settings = Settings;
    type Parameters = Parameters;
    type InputBindings = InputBindings;
    type OutputBindings = OutputBindings;

    fn new(context: &GpuContext, settings: Self::Settings) -> Self {
        let workgroup_size = settings.workgroup_size.get();
        let dispatch_limit = settings.dispatch_limit.get();
        let bit_count = settings.bit_count.get();
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size.is_multiple_of(subgroup_size));
        assert!(subgroup_size >= 2u32.pow(bit_count));

        let device = context.device();

        let_compiled_module!(
            count_subkeys,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 4,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size as f64),
                    ("BIT_COUNT", bit_count as f64),
                ],
            }
        );

        Self {
            workgroup_size,
            dispatch_limit,
            subgroup_size,
            bit_count,
            count_subkeys,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        allocator: &mut GpuAllocator,
        compute_pass: &mut wgpu::ComputePass,
        InputBindings {
            indirect,
            indices,
            keys,
        }: InputBindings,
        Parameters { bit_offset }: Parameters,
    ) -> Result<OutputBindings, GpuError> {
        assert_eq!(indices.len::<u32>(), keys.len::<u32>());

        let device = context.device();

        let counts_len = (self.min_counts_len(keys.len::<u32>().get() as u32) as u64)
            .try_into()
            .unwrap();
        let counts = allocator.allocate::<u32>("counts", counts_len)?;

        compute_pass.set_pipeline(&self.count_subkeys.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                device,
                &self.count_subkeys,
                [
                    indirect.binding(),
                    indices.binding(),
                    keys.binding(),
                    counts.binding(),
                ],
            ),
            &[],
        );
        compute_pass.set_immediates(0, bytemuck::bytes_of(&bit_offset));
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());
        Ok(OutputBindings { counts })
    }
}

impl CountSubkeys {
    pub fn min_counts_len(&self, key_len: u32) -> u32 {
        let subgroups_per_workgroup = self.workgroup_size / self.subgroup_size;
        let actual_workgroup_count = Indirect::new(IndirectSettings {
            workgroup_size: self.workgroup_size.try_into().unwrap(),
            dispatch_limit: self.dispatch_limit.try_into().unwrap(),
            len: key_len,
        })
        .workgroup_count();
        actual_workgroup_count * subgroups_per_workgroup * 2u32.pow(self.bit_count)
    }
}
