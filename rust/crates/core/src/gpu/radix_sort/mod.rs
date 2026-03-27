// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

// This implementation of radix sort is heavily inspired by
// Harada, Takahiro, and Lee Howes. "Introduction to GPU radix sort." Heterogeneous Computing with OpenCL. Morgan Kaufman (2011).

use super::*;

#[cfg(test)]
mod test;

pub struct RadixSort {
    bit_count: u32,
    count_subkeys: CountSubkeys,
    prefix_sum: PrefixSum,
    reorder: Reorder,
}

pub struct RadixSortSettings {
    pub count_subkeys_settings: CountSubkeysSettings,
    pub prefix_sum_settings: PrefixSumSettings,
    pub reorder_settings: ReorderSettings,
}

pub struct RadixSortBufferBindings<'a> {
    pub keys: wgpu::BufferBinding<'a>,
    pub indices: DoubleBuffer<'a>,

    pub counts: wgpu::BufferBinding<'a>,
    pub prefixes: wgpu::BufferBinding<'a>,
}

impl RadixSort {
    pub fn new(
        context: &GpuContext,
        RadixSortSettings {
            count_subkeys_settings,
            prefix_sum_settings,
            reorder_settings,
        }: RadixSortSettings,
    ) -> Self {
        assert_eq!(count_subkeys_settings.bit_count, reorder_settings.bit_count);
        let bit_count = count_subkeys_settings.bit_count;

        let count_subkeys = CountSubkeys::new(context, count_subkeys_settings);
        let prefix_sum = PrefixSum::new(context, prefix_sum_settings);
        let reorder = Reorder::new(context, reorder_settings);

        Self {
            bit_count,
            count_subkeys,
            prefix_sum,
            reorder,
        }
    }

    pub fn min_counts(&self, key_count: u32) -> u32 {
        self.count_subkeys.min_counts(key_count)
    }
    pub fn min_prefixes(&self, key_count: u32) -> u32 {
        self.reorder.min_prefixes(key_count)
    }

    pub fn compute_in_pass(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        RadixSortBufferBindings {
            keys,
            mut indices,
            counts,
            prefixes,
        }: RadixSortBufferBindings,
    ) -> bool {
        for round in 0..32u32.div_ceil(self.bit_count) {
            let bit_offset = round * self.bit_count;

            self.count_subkeys.compute_in_pass(
                context,
                compute_pass,
                indices.front(),
                keys.clone(),
                counts.clone(),
                bit_offset,
            );
            self.prefix_sum.compute_in_pass(
                context,
                compute_pass,
                counts.clone(),
                prefixes.clone(),
            );
            self.reorder.compute_in_pass(
                context,
                compute_pass,
                ReorderBufferBindings {
                    keys: keys.clone(),
                    prefixes: prefixes.clone(),
                    indices_in: indices.front(),
                    indices_out: indices.back(),
                },
                bit_offset,
            );

            indices.swap();
        }

        !indices.swapped()
    }
}
