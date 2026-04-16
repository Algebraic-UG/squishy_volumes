// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{cell::RefCell, rc::Rc, sync::atomic::AtomicU32};

use nalgebra::Vector4;
use wgpu::util::DeviceExt as _;

#[cfg(test)]
mod test;

use super::*;

pub struct PrepareGrid {
    sort_positions_into_cells: SortPositionsIntoCells,
    permute_positions: PermutePositions,
    find_cell_boundaries: FindCellBoundaries,
    prefix_sum: PrefixSum,
    build_cells: BuildCells,
    offsets_to_indirect: OffsetsToIndirect,
    color_cells: ColorCells2,
    reorder_particles: ReorderParticles,
    build_hash_table_colors: BuildHashTableColors,
    allocate_blocks: AllocateBlocks,
}

pub struct PrepareGridSettings {
    pub sort_positions_into_cells: SortPositionsIntoCellsSettings,
    pub permute_positions: PermutePositionsSettings,
    pub find_cell_boundaries: FindCellBoundariesSettings,
    pub prefix_sum: PrefixSumSettings,
    pub build_cells: BuildCellsSettings,
    pub offsets_to_indirect: OffsetsToIndirectSettings,
    pub color_cells: ColorCells2Settings,
    pub reorder_particles: ReorderParticlesSettings,
    pub build_hash_table_colors: BuildHashTableColorsSettings,
    pub allocate_blocks: AllocateBlocksSettings,
}

pub struct PrepareGridBufferInput<'a> {
    pub positions: &'a [Vector4<f32>],
    pub indices: &'a [u32],
}

pub struct PrepareGridBuffers {
    pub particle_positions_in: wgpu::Buffer,
    pub particle_indices_in: wgpu::Buffer,

    pub allocator: GpuAllocator,
}

pub struct PrepareGridBufferBindings<'a> {
    pub particle_positions_in: wgpu::BufferBinding<'a>,
    pub particle_indices_in: wgpu::BufferBinding<'a>,

    pub allocator: &'a mut GpuAllocator,

    pub cell_ids: Option<Allocation>,
    pub block_table: Option<Allocation>,
}

impl<'a> From<&'a PrepareGridBuffers> for PrepareGridBufferBindings<'a> {
    fn from(
        PrepareGridBuffers {
            particle_positions_in,
            particle_positions_out,
            particle_indices_front,
            particle_indices_back,
            particle_keys,
            particle_counts,
            particle_prefix_sums,
            particle_cell_boundaries,
            particle_cell_indices,
            cell_ids_in,
            cell_ids_out,
            cell_counts,
            cell_prefix_sums,
            cell_index_ranges,
            cell_owns,
            indirect,
            limits,
            block_table,
        }: &'a PrepareGridBuffers,
    ) -> Self {
        Self {
            particle_positions_in: particle_positions_in.as_entire_buffer_binding(),
            particle_positions_out: particle_positions_out.as_entire_buffer_binding(),
            particle_indices: Rc::new(RefCell::new(DoubleBuffer::new(
                particle_indices_front.as_entire_buffer_binding(),
                particle_indices_back.as_entire_buffer_binding(),
            ))),
            particle_keys: particle_keys.as_entire_buffer_binding(),
            particle_counts: particle_counts.as_entire_buffer_binding(),
            particle_prefix_sums: particle_prefix_sums.as_entire_buffer_binding(),
            particle_cell_boundaries: particle_cell_boundaries.as_entire_buffer_binding(),
            particle_cell_indices: particle_cell_indices.as_entire_buffer_binding(),
            cell_ids_in: cell_ids_in.as_entire_buffer_binding(),
            cell_ids_out: cell_ids_out.as_entire_buffer_binding(),
            cell_counts: cell_counts.as_entire_buffer_binding(),
            cell_prefix_sums: cell_prefix_sums.as_entire_buffer_binding(),
            cell_index_ranges: cell_index_ranges.as_entire_buffer_binding(),
            cell_owns: cell_owns.as_entire_buffer_binding(),
            indirect: indirect.as_entire_buffer_binding(),
            limits: limits.as_entire_buffer_binding(),
            block_table: block_table.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for PrepareGrid {
    type Settings = PrepareGridSettings;
    type Parameters = ();
    type BufferInput<'a> = PrepareGridBufferInput<'a>;
    type Buffers = PrepareGridBuffers;
    type BufferBindings<'a> = PrepareGridBufferBindings<'a>;

    fn new(
        context: &GpuContext,
        Self::Settings {
            sort_positions_into_cells,
            permute_positions,
            find_cell_boundaries,
            prefix_sum,
            build_cells,
            offsets_to_indirect,
            color_cells,
            build_hash_table_colors,
            allocate_blocks,
        }: Self::Settings,
    ) -> Self {
        Self {
            sort_positions_into_cells: SortPositionsIntoCells::new(
                context,
                sort_positions_into_cells,
            ),
            permute_positions: PermutePositions::new(context, permute_positions),
            find_cell_boundaries: FindCellBoundaries::new(context, find_cell_boundaries),
            prefix_sum: PrefixSum::new(context, prefix_sum),
            build_cells: BuildCells::new(context, build_cells),
            offsets_to_indirect: OffsetsToIndirect::new(context, offsets_to_indirect),
            color_cells: ColorCells2::new(context, color_cells),
            build_hash_table_colors: BuildHashTableColors::new(context, build_hash_table_colors),
            allocate_blocks: AllocateBlocks::new(context, allocate_blocks),
        }
    }

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput { positions, indices }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        assert_eq!(positions.len(), indices.len());
        let particle_n = positions.len() as u32;
        let cell_n = particle_n;

        let particle_counts_n = self
            .sort_positions_into_cells
            .min_counts_and_prefixes(particle_n);

        let cell_counts_n = self.color_cells.min_counts_and_prefixes(cell_n).max(cell_n);

        let block_table_n = self.build_hash_table_colors.max_table(cell_n);

        let device = context.device();

        let particle_positions_in = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle_positions"),
            contents: bytemuck::cast_slice(positions),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let particle_indices_front = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle_indices_front"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let_buffer!(device, particle_positions_out<Vector4<f32>>(particle_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, particle_indices_back<u32>(particle_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, particle_keys<u32>(particle_n, wgpu::BufferUsages::STORAGE));

        let_buffer!(device, particle_counts<u32>(particle_counts_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, particle_prefix_sums<u32>(particle_counts_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, particle_cell_boundaries<u32>(particle_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, particle_cell_indices<u32>(particle_n, wgpu::BufferUsages::STORAGE));

        let_buffer!(device, cell_ids_in<Vector4<i32>>(cell_n, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));
        let_buffer!(device, cell_ids_out<Vector4<i32>>(cell_n, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));
        let_buffer!(device, cell_counts<u32>(cell_counts_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, cell_prefix_sums<u32>(cell_counts_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, cell_index_ranges<u32>(cell_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, cell_owns<u32>(cell_n, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));

        let_buffer!(device, indirect<u32>(8 * 3, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::INDIRECT));
        let_buffer!(device, limits<u32>(8, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));

        let_buffer!(device, block_table<AtomicU32>(block_table_n, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));

        Self::Buffers {
            particle_positions_in,
            particle_positions_out,
            particle_indices_front,
            particle_indices_back,
            particle_keys,
            particle_counts,
            particle_prefix_sums,
            particle_cell_boundaries,
            particle_cell_indices,
            cell_ids_in,
            cell_ids_out,
            cell_counts,
            cell_prefix_sums,
            cell_index_ranges,
            cell_owns,
            indirect,
            limits,
            block_table,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            particle_positions_in,
            particle_positions_out,
            particle_indices,
            particle_keys,
            particle_counts,
            particle_prefix_sums,
            particle_cell_boundaries,
            particle_cell_indices,
            cell_ids_in,
            cell_ids_out,
            cell_counts,
            cell_prefix_sums,
            cell_index_ranges,
            cell_owns,
            indirect,
            limits,
            block_table,
        }: Self::BufferBindings<'a>,
        _: Self::Parameters,
    ) {
        self.sort_positions_into_cells.compute_in_pass(
            context,
            compute_pass,
            SortPositionsIntoCellsBufferBindings {
                positions: particle_positions_in.clone(),
                radix_sort: RadixSortBufferBindings {
                    keys: particle_keys.clone(),
                    indices: particle_indices.clone(),
                    counts: particle_counts.clone(),
                    prefix_sums: particle_prefix_sums.clone(),
                },
            },
            (),
        );

        self.permute_positions.compute_in_pass(
            context,
            compute_pass,
            PermutePositionsBufferBindings {
                permutation: particle_indices.borrow().back().clone(),
                positions_in: particle_positions_in,
                positions_out: particle_positions_out.clone(),
            },
            (),
        );

        self.find_cell_boundaries.compute_in_pass(
            context,
            compute_pass,
            FindCellBoundariesBufferBindings {
                positions: particle_positions_out.clone(),
                boundaries: particle_cell_boundaries.clone(),
            },
            (),
        );

        self.prefix_sum.compute_in_pass(
            context,
            compute_pass,
            PrefixSumBufferBindings {
                numbers: particle_cell_boundaries,
                prefix_sums: particle_cell_indices.clone(),
            },
            (),
        );

        self.build_cells.compute_in_pass(
            context,
            compute_pass,
            BuildCellsBufferBindings {
                positions: particle_positions_out.clone(),
                prefixed_boundaries: particle_cell_indices.clone(),
                cells: cell_ids_in.clone(),
                index_ranges: cell_index_ranges,
            },
            (),
        );

        self.offsets_to_indirect.compute_in_pass(
            context,
            compute_pass,
            OffsetsToIndirectBufferBindings {
                prefix_sums: particle_cell_indices,
                limits: limits.clone(),
                indirect: indirect.clone(),
            },
            (),
        );

        self.color_cells.compute_in_pass(
            context,
            compute_pass,
            ColorCells2BufferBindings {
                cells_in: cell_ids_in,
                cells_out: cell_ids_out.clone(),
                counts: cell_counts.clone(),
                prefix_sums: cell_prefix_sums.clone(),
                indirect: indirect.clone(),
                limits: limits.clone(),
            },
            (),
        );

        self.build_hash_table_colors.compute_in_pass(
            context,
            compute_pass,
            BuildHashTableColorsBufferBindings {
                cells: cell_ids_out,
                limits: limits.clone(),
                indirect,
                slots: block_table,
                owns: cell_owns.clone(),
            },
            (),
        );

        self.allocate_blocks.compute_in_pass(
            context,
            compute_pass,
            AllocateBlocksBufferBindings {
                owns: cell_owns,
                prefix_sum: PrefixSumBufferBindings {
                    numbers: cell_counts,
                    prefix_sums: cell_prefix_sums,
                },
            },
            (),
        );
    }
}
