// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

// This implementation of radix sort is heavily inspired by
// Harada, Takahiro, and Lee Howes. "Introduction to GPU radix sort." Heterogeneous Computing with OpenCL. Morgan Kaufman (2011).

use std::num::NonZeroU32;

use super::*;

#[cfg(test)]
mod test;

pub struct RadixSort {
    bit_count: u32,
    count_subkeys: CountSubkeys,
    counts_indirect: CountsIndirect,
    prefix_sum: PrefixSum,
    reorder_indices: ReorderIndices,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub bit_count: NonZeroU32,
}

pub struct Parameters {
    pub bit_offset: u32,
}

pub struct Input {
    pub indirect: Allocation,
    pub indices_in: Option<Allocation>,
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

        let indices_in = indices
            .map(|indices| Allocation::new(device, "indices_in", indices))
            .transpose()?;
        let keys = Allocation::new(device, "keys", keys)?;
        let indirect = Allocation::new(device, "indirect", &[indirect])?;

        Ok(Self {
            indirect,
            indices_in,
            keys,
        })
    }
}

pub struct Output {
    pub indices_out: Allocation,
    pub prefix_sums: Allocation,
}

impl PipelinePart for RadixSort {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            bit_count,
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let count_subkeys = CountSubkeys::new(
            context,
            count_subkeys::Settings {
                workgroup_size,
                dispatch_limit,
                bit_count,
            },
        )?;
        let counts_indirect = CountsIndirect::new(
            context,
            counts_indirect::Settings {
                workgroup_size,
                dispatch_limit,
                bit_count,
            },
        )?;
        let prefix_sum = PrefixSum::new(
            context,
            prefix_sum::Settings {
                workgroup_size,
                dispatch_limit,
            },
        )?;
        let reorder_indices = ReorderIndices::new(
            context,
            reorder_indices::Settings {
                workgroup_size,
                dispatch_limit,
                bit_count,
            },
        )?;

        count_subkeys
            .representative_module()
            .check_same_sugroup_size(counts_indirect.representative_module())?;
        count_subkeys
            .representative_module()
            .check_same_sugroup_size(prefix_sum.representative_module())?;
        count_subkeys
            .representative_module()
            .check_same_sugroup_size(reorder_indices.representative_module())?;

        Ok(Self {
            bit_count: bit_count.get(),
            count_subkeys,
            counts_indirect,
            prefix_sum,
            reorder_indices,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            keys,
            indices_in,
        }: Input,
        Parameters { bit_offset }: Parameters,
    ) -> Result<Output, GpuError> {
        let count_subkeys::Output { counts } = self.count_subkeys.record(
            context,
            encoder,
            count_subkeys::Input {
                indirect: indirect.clone(),
                indices: indices_in.clone(),
                keys: keys.clone(),
            },
            count_subkeys::Parameters { bit_offset },
        )?;
        let counts_indirect::Output { indirect_counts } = self.counts_indirect.record(
            context,
            encoder,
            counts_indirect::Input {
                indirect: indirect.clone(),
            },
            counts_indirect::Parameters,
        )?;
        let prefix_sum::Output {
            prefix_sums,
            total_sum,
        } = self.prefix_sum.record(
            context,
            encoder,
            prefix_sum::Input {
                indirect: indirect_counts,
                numbers: counts,
            },
            prefix_sum::Parameters { total_sum: false },
        )?;
        assert!(total_sum.is_none());
        let reorder_indices::Output { indices_out } = self.reorder_indices.record(
            context,
            encoder,
            reorder_indices::Input {
                indirect,
                indices_in,
                keys,
                prefix_sums: prefix_sums.clone(),
            },
            reorder_indices::Parameters { bit_offset },
        )?;

        Ok(Output {
            indices_out,
            prefix_sums,
        })
    }
}

impl RadixSort {
    pub fn record_all_rounds(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            indices_in,
            keys,
        }: Input,
    ) -> Result<Allocation, GpuError> {
        let mut indices_out = indices_in;
        for round in 0..32u32.div_ceil(self.bit_count) {
            let output = self.record(
                context,
                encoder,
                Input {
                    indirect: indirect.clone(),
                    indices_in: indices_out,
                    keys: keys.clone(),
                },
                Parameters {
                    bit_offset: round * self.bit_count,
                },
            )?;
            indices_out = Some(output.indices_out);
        }

        Ok(indices_out.unwrap())
    }
}
