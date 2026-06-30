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

pub struct ReorderIndices {
    workgroup_size: u32,
    dispatch_limit: u32,
    subgroup_size: u32,
    bit_count: u32,
    reorder_indices: CompiledModule,
    reorder_indices_with_indices: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub bit_count: NonZeroU32,
}

#[derive(Clone, Copy)]
pub struct Parameters {
    pub bit_offset: u32,
}

pub struct Input {
    pub indirect: Allocation,
    pub indices_in: Option<Allocation>,
    pub keys: Allocation,
    pub prefix_sums: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
            ..
        }: Settings,
        indices: Option<&[u32]>,
        keys: &[u32],
        prefix_sums: &[u32],
    ) -> Result<Self, GpuError> {
        if let Some(indices) = indices.as_ref() {
            check_length!(indices, keys)?;
        }

        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: keys.len() as u32,
        });

        let indices_in = indices
            .map(|indices| Allocation::new(device, "indices_in", indices))
            .transpose()?;
        let keys = Allocation::new(device, "keys", keys)?;
        let prefix_sums = Allocation::new(device, "prefix_sums", prefix_sums)?;
        let indirect = Allocation::new(device, "indirect", &[indirect])?;

        Ok(Self {
            indirect,
            indices_in,
            keys,
            prefix_sums,
        })
    }
}

pub struct Output {
    pub indices_out: Allocation,
}

impl PipelinePart for ReorderIndices {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &GpuContext, settings: Self::Settings) -> Self {
        let workgroup_size = settings.workgroup_size.get();
        let dispatch_limit = settings.dispatch_limit.get();
        let bit_count = settings.bit_count.get();
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size.is_multiple_of(subgroup_size));
        assert!(subgroup_size >= 2u32.pow(bit_count));

        let device = context.device();

        let_compiled_module!(
            reorder_indices,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 4,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size as f64),
                    ("BIT_COUNT", bit_count as f64),
                ],
            }
        );

        let_compiled_module!(
            reorder_indices_with_indices,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
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
            reorder_indices,
            reorder_indices_with_indices,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            indices_in,
            keys,
            prefix_sums,
        }: Input,
        Parameters { bit_offset }: Parameters,
    ) -> Result<Output, GpuError> {
        if let Some(indices_in) = indices_in.as_ref() {
            assert_eq!(indices_in.len::<u32>(), keys.len::<u32>());
        }
        assert!(
            prefix_sums.len::<u32>().get()
                >= self.min_prefix_sums_len(keys.len::<u32>().get() as u32) as u64
        );

        let indices_out = context
            .allocator()?
            .allocate::<u32>("indices_out", keys.len::<u32>())?;

        let mut compute_pass = if let Some(indices_in) = indices_in.as_ref() {
            context.enter_module(
                encoder,
                &self.reorder_indices_with_indices,
                [
                    indirect.binding(),
                    keys.binding(),
                    prefix_sums.binding(),
                    indices_in.binding(),
                    indices_out.binding(),
                ],
            )
        } else {
            context.enter_module(
                encoder,
                &self.reorder_indices,
                [
                    indirect.binding(),
                    keys.binding(),
                    prefix_sums.binding(),
                    indices_out.binding(),
                ],
            )
        };
        compute_pass.set_immediates(0, bytemuck::bytes_of(&bit_offset));
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());
        drop(compute_pass);

        Ok(Output { indices_out })
    }
}

impl ReorderIndices {
    pub fn min_prefix_sums_len(&self, len: u32) -> u32 {
        counts_count(CountsCountArgs {
            workgroup_size: self.workgroup_size,
            subgroup_size: self.subgroup_size,
            dispatch_limit: self.dispatch_limit,
            counter: 2u32.pow(self.bit_count),
            len,
        })
    }
}
