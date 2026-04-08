// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use nalgebra::Vector4;
use wgpu::util::DeviceExt as _;

use super::*;

pub struct ColorCells {
    cells_to_colorkeys: CellsToColorkeys,
    radix_sort: RadixSort,
    recycle_to_indirect: RecycleToIndirect,
}

pub struct ColorCellsSettings {
    pub workgroup_size: u32,
}

pub struct ColorCellsBufferInput<'a> {
    pub cells: &'a [Vector4<i32>],
}

pub struct ColorCellsBuffers {
    pub cells: wgpu::Buffer,
    pub indirect: wgpu::Buffer,
    pub limits: wgpu::Buffer,
    pub radix_sort: RadixSortBuffers,
}

pub struct ColorCellsBufferBindings<'a> {
    pub cells: wgpu::BufferBinding<'a>,
    pub indirect: wgpu::BufferBinding<'a>,
    pub limits: wgpu::BufferBinding<'a>,
    pub radix_sort: RadixSortBufferBindings<'a>,
}

impl<'a> From<&'a ColorCellsBuffers> for ColorCellsBufferBindings<'a> {
    fn from(
        ColorCellsBuffers {
            cells,
            indirect,
            limits,
            radix_sort,
        }: &'a ColorCellsBuffers,
    ) -> Self {
        Self {
            cells: cells.as_entire_buffer_binding(),
            indirect: indirect.as_entire_buffer_binding(),
            limits: limits.as_entire_buffer_binding(),
            radix_sort: radix_sort.into(),
        }
    }
}

impl PipelinePart for ColorCells {
    type Settings = ColorCellsSettings;
    type Parameters = ();
    type BufferInput<'a> = ColorCellsBufferInput<'a>;
    type Buffers = ColorCellsBuffers;
    type BufferBindings<'a> = ColorCellsBufferBindings<'a>;

    fn new(context: &GpuContext, Self::Settings { workgroup_size }: Self::Settings) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size > 0);
        assert!(workgroup_size.is_multiple_of(subgroup_size));

        // one per dimension
        let bit_count = 3;

        let cells_to_colorkeys =
            CellsToColorkeys::new(context, CellsToColorkeysSettings { workgroup_size });

        let radix_sort = RadixSort::new(
            context,
            RadixSortSettings {
                count_subkeys_settings: CountSubkeysSettings {
                    workgroup_size,
                    bit_count,
                },
                prefix_sum_settings: PrefixSumSettings { workgroup_size },
                reorder_settings: ReorderSettings {
                    workgroup_size,
                    bit_count,
                },
            },
        );

        let recycle_to_indirect = RecycleToIndirect::new(
            context,
            RecycleToIndirectSettings {
                workgroup_size,
                dispatch_limit: context
                    .adapter()
                    .limits()
                    .max_compute_workgroups_per_dimension,
            },
        );

        Self {
            cells_to_colorkeys,
            radix_sort,
            recycle_to_indirect,
        }
    }

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput { cells }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let device = context.device();
        let n = cells.len();

        let cells = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cells"),
            contents: bytemuck::cast_slice(cells),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let indirect = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("indirect"),
            size: 8 * 3 * u32::MIN_BINDING_SIZE.get(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let limits = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("limits"),
            size: 8 * u32::MIN_BINDING_SIZE.get(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let radix_sort = self.radix_sort.create_buffers(
            context,
            RadixSortBufferInput {
                keys: &vec![0; n],
                indices: &(0..n as u32).collect::<Vec<_>>(),
            },
        );

        Self::Buffers {
            cells,
            indirect,
            limits,
            radix_sort,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            cells,
            indirect,
            limits,
            radix_sort,
        }: &mut Self::BufferBindings<'a>,
        _: &mut Self::Parameters,
    ) {
        self.cells_to_colorkeys.compute_in_pass(
            context,
            compute_pass,
            &mut CellsToColorkeysBufferBindings {
                cells: cells.clone(),
                keys: radix_sort.keys.clone(),
            },
            &mut (),
        );
        self.radix_sort.compute_in_pass(
            context,
            compute_pass,
            radix_sort,
            &mut RadixSortParamters { bit_offset: 0 },
        );
        self.recycle_to_indirect.compute_in_pass(
            context,
            compute_pass,
            &mut RecycleToIndirectBufferBindings {
                indirect: indirect.clone(),
                limits: limits.clone(),
                prefix_sums: radix_sort.prefix_sums.clone(),
            },
            &mut (),
        );
    }
}

impl ColorCells {
    pub fn min_counts_and_prefixes(&self, key_count: u32) -> u32 {
        self.radix_sort.min_counts_and_prefixes(key_count)
    }
}
