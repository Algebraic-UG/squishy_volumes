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
    pub indices_in: Allocation,
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
        indices: &[u32],
        keys: &[u32],
    ) -> Self {
        assert_eq!(indices.len(), keys.len());
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: indices.len() as u32,
        });

        let indices_in = Allocation::new(device, "indices_in", indices);
        let keys = Allocation::new(device, "keys", keys);
        let indirect = Allocation::new(device, "indirect", &[indirect]);

        Self {
            indirect,
            indices_in,
            keys,
        }
    }
}

pub struct Output {
    pub indices_out: Allocation,
}

impl PipelinePart for RadixSort {
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
        let count_subkeys = CountSubkeys::new(
            context,
            count_subkeys::Settings {
                workgroup_size,
                dispatch_limit,
                bit_count,
            },
        );
        let prefix_sum = PrefixSum::new(
            context,
            prefix_sum::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );
        let reorder_indices = ReorderIndices::new(
            context,
            reorder_indices::Settings {
                workgroup_size,
                dispatch_limit,
                bit_count,
            },
        );

        Self {
            bit_count: bit_count.get(),
            count_subkeys,
            prefix_sum,
            reorder_indices,
        }
    }

    fn encode(
        &self,
        context: &mut GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Input {
            indirect,
            keys,
            indices_in,
        }: Input,
        Parameters { bit_offset }: Parameters,
    ) -> Result<Output, GpuError> {
        let count_subkeys::Output { counts } = self.count_subkeys.encode(
            context,
            compute_pass,
            count_subkeys::Input {
                indirect: indirect.clone(),
                indices: indices_in.clone(),
                keys: keys.clone(),
            },
            count_subkeys::Parameters { bit_offset },
        )?;
        let prefix_sum::Output { prefix_sums } = self.prefix_sum.encode(
            context,
            compute_pass,
            prefix_sum::Input {
                indirect: indirect.clone(),
                numbers: counts,
            },
            prefix_sum::Parameters,
        )?;
        let reorder_indices::Output { indices_out } = self.reorder_indices.encode(
            context,
            compute_pass,
            reorder_indices::Input {
                indirect,
                indices_in,
                keys,
                prefix_sums,
            },
            reorder_indices::Parameters { bit_offset },
        )?;

        Ok(Output { indices_out })
    }
}

impl RadixSort {
    pub fn compute_in_pass_all_rounds(
        &self,
        context: &mut GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Input {
            indirect,
            mut indices_in,
            keys,
        }: Input,
    ) -> Result<Output, GpuError> {
        for round in 0..32u32.div_ceil(self.bit_count) {
            let Output { indices_out } = self.encode(
                context,
                compute_pass,
                Input {
                    indirect: indirect.clone(),
                    indices_in,
                    keys: keys.clone(),
                },
                Parameters {
                    bit_offset: round * self.bit_count,
                },
            )?;
            indices_in = indices_out;
        }

        Ok(Output {
            indices_out: indices_in,
        })
    }
}
