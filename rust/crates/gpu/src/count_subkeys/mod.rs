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

pub struct CountSubkeys {
    workgroup_size: u32,
    dispatch_limit: u32,
    subgroup_size: u32,
    bit_count: u32,
    count_subkeys: CompiledModule,
    count_subkeys_with_indices: CompiledModule,
}

impl CountSubkeys {
    pub fn count_subkeys(&self) -> &CompiledModule {
        &self.count_subkeys
    }
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
    pub indices: Option<Allocation>,
    pub keys: Allocation,
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
    ) -> Result<Self, GpuError> {
        if let Some(indices) = indices.as_ref() {
            check_length!(indices, keys)?;
        }

        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: keys.len() as u32,
        });

        let indices = indices
            .map(|indices| Allocation::new(device, "indices", indices))
            .transpose()?;
        let keys = Allocation::new(device, "keys", keys)?;
        let indirect = Allocation::new(device, "indirect", &[indirect])?;

        Ok(Self {
            indirect,
            indices,
            keys,
        })
    }
}

pub struct Output {
    pub counts: Allocation,
}

impl PipelinePart for CountSubkeys {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        settings: Self::Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let workgroup_size = settings.workgroup_size.get();
        let dispatch_limit = settings.dispatch_limit.get();
        let bit_count = settings.bit_count.get();

        let_compiled_module!(
            count_subkeys,
            CompiledModuleSettings {
                context,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
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
            count_subkeys_with_indices,
            CompiledModuleSettings {
                context,
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

        count_subkeys.check_same_sugroup_size(&count_subkeys_with_indices)?;
        count_subkeys.check_workgroup_size_multiple_of_subgroup_size(workgroup_size)?;
        count_subkeys.check_subgroup_size_at_least(2u32.pow(bit_count))?;

        let subgroup_size = count_subkeys.subgroup_size.get();

        Ok(Self {
            workgroup_size,
            dispatch_limit,
            subgroup_size,
            bit_count,
            count_subkeys,
            count_subkeys_with_indices,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            indices,
            keys,
        }: Input,
        Parameters { bit_offset }: Parameters,
    ) -> Result<Output, GpuError> {
        if let Some(indices) = indices.as_ref() {
            assert_eq!(indices.len::<u32>(), keys.len::<u32>());
        }

        let counts_len = (self.min_counts_len(keys.len::<u32>().get() as u32) as u64)
            .try_into()
            .unwrap();
        let counts = context.allocator()?.allocate::<u32>("counts", counts_len)?;

        let mut compute_pass = if let Some(indices) = indices.as_ref() {
            context.enter_module(
                encoder,
                &self.count_subkeys_with_indices,
                [
                    indirect.binding(),
                    indices.binding(),
                    keys.binding(),
                    counts.binding(),
                ],
            )
        } else {
            context.enter_module(
                encoder,
                &self.count_subkeys,
                [indirect.binding(), keys.binding(), counts.binding()],
            )
        };
        compute_pass.set_immediates(0, bytemuck::bytes_of(&bit_offset));
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());
        drop(compute_pass);
        Ok(Output { counts })
    }
}

impl CountSubkeys {
    pub fn min_counts_len(&self, len: u32) -> u32 {
        counts_count(CountsCountArgs {
            workgroup_size: self.workgroup_size,
            subgroup_size: self.subgroup_size,
            dispatch_limit: self.dispatch_limit,
            counter: 2u32.pow(self.bit_count),
            len,
        })
    }
}
