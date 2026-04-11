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
    generate_indices: GenerateIndices,
    color_cells: ColorCells,
    permute_cells: PermuteCells,
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
    pub generate_indices: GenerateIndicesSettings,
    pub color_cells: ColorCellsSettings,
    pub permute_cells: PermuteCellsSettings,
    pub build_hash_table_colors: BuildHashTableColorsSettings,
    pub allocate_blocks: AllocateBlocksSettings,
}

pub struct PrepareGridBufferInput<'a> {
    pub positions: &'a [Vector4<f32>],
    pub indices: &'a [u32],
}

pub struct PrepareGridBuffers {
    pub particle_positions_in: wgpu::Buffer,
    pub particle_positions_out: wgpu::Buffer,

    pub particle_indices_front: wgpu::Buffer,
    pub particle_indices_back: wgpu::Buffer,

    pub particle_keys: wgpu::Buffer,
    pub particle_counts: wgpu::Buffer,
    pub particle_prefix_sums: wgpu::Buffer,

    pub cell_ids_in: wgpu::Buffer,
    pub cell_ids_out: wgpu::Buffer,

    pub cell_indices_front: wgpu::Buffer,
    pub cell_indices_back: wgpu::Buffer,

    pub cell_keys: wgpu::Buffer,
    pub cell_counts: wgpu::Buffer,
    pub cell_prefix_sums: wgpu::Buffer,

    pub cell_index_ranges: wgpu::Buffer,
    pub cell_owns: wgpu::Buffer,

    pub indirect: wgpu::Buffer,
    pub limits: wgpu::Buffer,

    pub block_table: wgpu::Buffer,
}

pub struct PrepareGridBufferBindings<'a> {
    pub particle_positions_in: wgpu::BufferBinding<'a>,
    pub particle_positions_out: wgpu::BufferBinding<'a>,

    pub particle_indices: Rc<RefCell<DoubleBuffer<'a>>>,

    pub particle_keys: wgpu::BufferBinding<'a>,
    pub particle_counts: wgpu::BufferBinding<'a>,
    pub particle_prefix_sums: wgpu::BufferBinding<'a>,

    pub cell_ids_in: wgpu::BufferBinding<'a>,
    pub cell_ids_out: wgpu::BufferBinding<'a>,

    pub cell_indices: Rc<RefCell<DoubleBuffer<'a>>>,

    pub cell_keys: wgpu::BufferBinding<'a>,
    pub cell_counts: wgpu::BufferBinding<'a>,
    pub cell_prefix_sums: wgpu::BufferBinding<'a>,

    pub cell_index_ranges: wgpu::BufferBinding<'a>,
    pub cell_owns: wgpu::BufferBinding<'a>,

    pub indirect: wgpu::BufferBinding<'a>,
    pub limits: wgpu::BufferBinding<'a>,

    pub block_table: wgpu::BufferBinding<'a>,
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
            cell_ids_in,
            cell_ids_out,
            cell_indices_front,
            cell_indices_back,
            cell_keys,
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
            cell_ids_in: cell_ids_in.as_entire_buffer_binding(),
            cell_ids_out: cell_ids_out.as_entire_buffer_binding(),
            cell_indices: Rc::new(RefCell::new(DoubleBuffer::new(
                cell_indices_front.as_entire_buffer_binding(),
                cell_indices_back.as_entire_buffer_binding(),
            ))),
            cell_keys: cell_keys.as_entire_buffer_binding(),
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
            generate_indices,
            color_cells,
            permute_cells,
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
            generate_indices: GenerateIndices::new(context, generate_indices),
            color_cells: ColorCells::new(context, color_cells),
            permute_cells: PermuteCells::new(context, permute_cells),
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

        let cell_counts_n = self.color_cells.min_counts_and_prefixes(cell_n);

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

        let_buffer!(device, cell_ids_in<Vector4<i32>>(cell_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, cell_ids_out<Vector4<i32>>(cell_n, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));
        let_buffer!(device, cell_indices_front<u32>(cell_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, cell_indices_back<u32>(cell_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, cell_keys<u32>(cell_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, cell_index_ranges<u32>(cell_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, cell_owns<u32>(cell_n, wgpu::BufferUsages::STORAGE));

        let_buffer!(device, cell_counts<u32>(cell_counts_n, wgpu::BufferUsages::STORAGE));
        let_buffer!(device, cell_prefix_sums<u32>(cell_counts_n, wgpu::BufferUsages::STORAGE));

        let_buffer!(device, indirect<u32>(8 * 3, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT));
        let_buffer!(device, limits<u32>(8, wgpu::BufferUsages::STORAGE));

        let_buffer!(device, block_table<AtomicU32>(block_table_n, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));

        Self::Buffers {
            particle_positions_in,
            particle_positions_out,
            particle_indices_front,
            particle_indices_back,
            particle_keys,
            particle_counts,
            particle_prefix_sums,
            cell_ids_in,
            cell_ids_out,
            cell_indices_front,
            cell_indices_back,
            cell_keys,
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
            cell_ids_in,
            cell_ids_out,
            cell_indices,
            cell_keys,
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
                boundaries: particle_keys.clone(), // ugh
            },
            (),
        );

        self.prefix_sum.compute_in_pass(
            context,
            compute_pass,
            PrefixSumBufferBindings {
                numbers: particle_keys.clone(),                 // double ugh
                prefix_sums: particle_indices.borrow().front(), // uggh
            },
            (),
        );

        self.build_cells.compute_in_pass(
            context,
            compute_pass,
            BuildCellsBufferBindings {
                positions: particle_positions_out.clone(),
                prefixed_boundaries: particle_indices.borrow().front(), // uggh
                cells: cell_ids_in.clone(),
                index_ranges: cell_index_ranges,
            },
            (),
        );

        self.offsets_to_indirect.compute_in_pass(
            context,
            compute_pass,
            OffsetsToIndirectBufferBindings {
                prefix_sums: particle_indices.borrow().front(),
                limits: limits.clone(),
                indirect: indirect.clone(),
            },
            (),
        );

        self.generate_indices.compute_in_pass(
            context,
            compute_pass,
            GenerateIndicesBufferBindings {
                indices: cell_indices.borrow().front(),
                limits: limits.clone(),
                indirect: indirect.clone(),
            },
            (),
        );

        self.color_cells.compute_in_pass(
            context,
            compute_pass,
            ColorCellsBufferBindings {
                cells: cell_ids_in.clone(),
                indirect: indirect.clone(),
                limits: limits.clone(),
                radix_sort: RadixSortBufferBindings {
                    keys: cell_keys.clone(),
                    indices: cell_indices.clone(),
                    counts: cell_counts,
                    prefix_sums: cell_prefix_sums,
                },
            },
            (),
        );

        self.permute_cells.compute_in_pass(
            context,
            compute_pass,
            PermuteCellsBufferBindings {
                permutation: cell_indices.borrow().back(),
                cells_in: cell_ids_in,
                cells_out: cell_ids_out.clone(),
            },
            (),
        );

        self.build_hash_table_colors.compute_in_pass(
            context,
            compute_pass,
            BuildHashTableColorsBufferBindings {
                cells: cell_ids_out,
                indices: cell_indices.borrow().back(),
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
                    numbers: cell_keys,
                    prefix_sums: cell_indices.borrow().front(),
                },
            },
            (),
        );
    }
}
