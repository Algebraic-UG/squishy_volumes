// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

// This implementation of radix sort is heavily inspired by
// Harada, Takahiro, and Lee Howes. "Introduction to GPU radix sort." Heterogeneous Computing with OpenCL. Morgan Kaufman (2011).

use wgpu::util::DeviceExt as _;

use super::*;

#[cfg(test)]
mod test;

pub struct RadixSort {
    bit_count: u32,
    count_subkeys: CountSubkeys,
    prefix_sum: PrefixSum,
    reorder: Reorder,
}

#[derive(Clone, Copy)]
pub struct RadixSortSettings {
    pub count_subkeys_settings: CountSubkeysSettings,
    pub prefix_sum_settings: PrefixSumSettings,
    pub reorder_settings: ReorderSettings,
}

pub struct RadixSortParamters {
    pub bit_offset: u32,
}

pub struct RadixSortBufferInput<'a> {
    pub keys: &'a [u32],
    pub indices: &'a [u32],
}

pub struct RadixSortBuffers {
    pub keys: wgpu::Buffer,
    pub indices_front: wgpu::Buffer,
    pub indices_back: wgpu::Buffer,
    pub counts: wgpu::Buffer,
    pub prefix_sums: wgpu::Buffer,
}

pub struct RadixSortBufferBindings<'a> {
    pub keys: wgpu::BufferBinding<'a>,
    pub indices: DoubleBuffer<'a>,

    pub counts: wgpu::BufferBinding<'a>,
    pub prefix_sums: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a RadixSortBuffers> for RadixSortBufferBindings<'a> {
    fn from(
        RadixSortBuffers {
            keys,
            indices_front,
            indices_back,
            counts,
            prefix_sums,
        }: &'a RadixSortBuffers,
    ) -> Self {
        Self {
            keys: keys.as_entire_buffer_binding(),
            indices: DoubleBuffer::new(
                indices_front.as_entire_buffer_binding(),
                indices_back.as_entire_buffer_binding(),
            ),
            counts: counts.as_entire_buffer_binding(),
            prefix_sums: prefix_sums.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for RadixSort {
    type Settings = RadixSortSettings;
    type Parameters = RadixSortParamters;
    type BufferInput<'a> = RadixSortBufferInput<'a>;
    type Buffers = RadixSortBuffers;
    type BufferBindings<'a> = RadixSortBufferBindings<'a>;

    fn new(
        context: &GpuContext,
        Self::Settings {
            count_subkeys_settings,
            prefix_sum_settings,
            reorder_settings,
        }: Self::Settings,
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

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput { keys, indices }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        assert_eq!(indices.len(), keys.len());
        assert!(indices.len() < u32::MAX as usize);
        let n = indices.len() as u32;

        let count_size = self.min_counts_and_prefixes(n) * 4;
        let prefix_size = count_size;

        let device = context.device();

        let keys = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("keys"),
            contents: bytemuck::cast_slice(keys),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let indices_front = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indices_front"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        let indices_back = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("indices_back"),
            size: indices_front.size(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let counts = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("counts"),
            size: count_size as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let prefix_sums = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("prefix_sums"),
            size: prefix_size as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        Self::Buffers {
            keys,
            indices_front,
            indices_back,
            counts,
            prefix_sums,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            keys,
            indices,
            counts,
            prefix_sums,
        }: &Self::BufferBindings<'a>,
        Self::Parameters { bit_offset }: &Self::Parameters,
    ) {
        self.count_subkeys.compute_in_pass(
            context,
            compute_pass,
            &CountSubkeysBufferBindings {
                indices: indices.front(),
                keys: keys.clone(),
                counts: counts.clone(),
            },
            &CountSubkeysParamters {
                bit_offset: *bit_offset,
            },
        );
        self.prefix_sum.compute_in_pass(
            context,
            compute_pass,
            &PrefixSumBufferBindings {
                numbers: counts.clone(),
                prefix_sums: prefix_sums.clone(),
            },
            &(),
        );
        self.reorder.compute_in_pass(
            context,
            compute_pass,
            &ReorderBufferBindings {
                keys: keys.clone(),
                prefix_sums: prefix_sums.clone(),
                indices_in: indices.front(),
                indices_out: indices.back(),
            },
            &ReorderParameters {
                bit_offset: *bit_offset,
            },
        );
    }
}

impl RadixSort {
    pub fn min_counts_and_prefixes(&self, key_count: u32) -> u32 {
        self.count_subkeys.min_counts(key_count)
    }

    pub fn compute_in_pass_all_rounds<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        buffer_bindings: &RadixSortBufferBindings<'a>,
    ) {
        for round in 0..32u32.div_ceil(self.bit_count) {
            self.compute_in_pass(
                context,
                compute_pass,
                buffer_bindings,
                &RadixSortParamters {
                    bit_offset: round * self.bit_count,
                },
            );
            buffer_bindings.indices.swap();
        }
    }
}
