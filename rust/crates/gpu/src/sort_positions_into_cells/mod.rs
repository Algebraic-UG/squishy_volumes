// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector4;
use wgpu::util::DeviceExt as _;

use super::*;

#[cfg(test)]
mod test;

pub struct SortPositionsIntoCells {
    positions_to_keys: PositionsToKeys,
    radix_sort: RadixSort,
}

pub struct SortPositionsIntoCellsSettings {
    pub positions_to_keys_settings: PositionsToKeysSettings,
    pub radix_sort_setttings: RadixSortSettings,
}

pub struct SortPositionsIntoCellsBufferInput<'a> {
    pub indices: &'a [u32],
    pub positions: &'a [Vector4<f32>],
}

pub struct SortPositionsIntoCellsBuffers {
    pub positions: wgpu::Buffer,
    pub radix_sort: RadixSortBuffers,
}

pub struct SortPositionsIntoCellsBufferBindings<'a> {
    pub positions: wgpu::BufferBinding<'a>,
    pub radix_sort: RadixSortBufferBindings<'a>,
}

impl<'a> From<&'a SortPositionsIntoCellsBuffers> for SortPositionsIntoCellsBufferBindings<'a> {
    fn from(
        SortPositionsIntoCellsBuffers {
            positions,
            radix_sort,
        }: &'a SortPositionsIntoCellsBuffers,
    ) -> Self {
        Self {
            positions: positions.as_entire_buffer_binding(),
            radix_sort: radix_sort.into(),
        }
    }
}

impl PipelinePart for SortPositionsIntoCells {
    type Settings = SortPositionsIntoCellsSettings;
    type Parameters = ();
    type BufferInput<'a> = SortPositionsIntoCellsBufferInput<'a>;
    type Buffers = SortPositionsIntoCellsBuffers;
    type BufferBindings<'a> = SortPositionsIntoCellsBufferBindings<'a>;

    fn new(
        context: &GpuContext,
        Self::Settings {
            positions_to_keys_settings,
            radix_sort_setttings,
        }: Self::Settings,
    ) -> Self {
        let positions_to_keys = PositionsToKeys::new(context, positions_to_keys_settings);
        let radix_sort = RadixSort::new(context, radix_sort_setttings);

        Self {
            positions_to_keys,
            radix_sort,
        }
    }

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput { indices, positions }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        assert_eq!(indices.len(), positions.len());
        let device = context.device();
        let n = positions.len();

        let positions = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("positions"),
            contents: bytemuck::cast_slice(positions),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let radix_sort = self.radix_sort.create_buffers(
            context,
            RadixSortBufferInput {
                keys: &vec![0; n],
                indices,
            },
        );

        Self::Buffers {
            positions,
            radix_sort,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            positions,
            radix_sort,
        }: &mut Self::BufferBindings<'a>,
        _: &mut Self::Parameters,
    ) {
        for dimension in [0] {
            //, 1, 2] {
            self.positions_to_keys.compute_in_pass(
                context,
                compute_pass,
                &mut PositionsToKeysBufferBindings {
                    positions: positions.clone(),
                    keys: radix_sort.keys.clone(),
                },
                &mut PositionsToKeysParameters { dimension },
            );
            self.radix_sort
                .compute_in_pass(context, compute_pass, radix_sort, &mut ());
        }
    }
}
