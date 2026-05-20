// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU32;

use nalgebra::Vector4;

#[cfg(test)]
mod test;

use super::*;

pub struct PrepareGrid {
    find_cell_boundaries: FindCellBoundaries,
    prefix_sum: PrefixSum,
    build_cells: BuildCells,
    color_cells: ColorCells,
    build_hash_table_from_cells: BuildHashTableFromCells,
    allocate_blocks: AllocateBlocks,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub cell_size: f32,
}

pub struct Parameters;

pub struct Input {
    pub indirect_particles: Allocation,
    pub positions: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
            cell_size,
            ..
        }: Settings,
        positions: &[Vector4<f32>],
    ) -> Self {
        let permutation = sort_positions_into_cells_on_cpu(
            &(0..positions.len() as u32).collect::<Vec<_>>(),
            positions,
            cell_size,
        );
        let positions = permutation.as_slice().permute(positions);

        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: positions.len() as u32,
        });

        let indirect_particles = Allocation::new(device, "indirect_particles", &[indirect]);
        let positions = Allocation::new(device, "positions", &positions);

        Self {
            indirect_particles,
            positions,
        }
    }
}

pub struct Output {
    pub indirect_cells: Allocation,
    pub indirect_cells_batch: Allocation,
    pub indirect_colors: Allocation,
    pub indirect_colors_batch: Allocation,

    pub cell_indices: Allocation,
    pub cell_index_ranges: Allocation,
    pub cell_ids: Allocation,
    pub cell_owns: Allocation,
    pub block_offsets: Allocation,
    pub block_table: Allocation,
}

impl PipelinePart for PrepareGrid {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            cell_size,
        }: Settings,
    ) -> Self {
        let find_cell_boundaries = FindCellBoundaries::new(
            context,
            find_cell_boundaries::Settings {
                workgroup_size,
                cell_size,
            },
        );
        let prefix_sum = PrefixSum::new(
            context,
            prefix_sum::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );
        let build_cells = BuildCells::new(
            context,
            build_cells::Settings {
                workgroup_size,
                dispatch_limit,
                cell_size,
            },
        );
        let color_cells = ColorCells::new(
            context,
            color_cells::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );
        let build_hash_table_from_cells = BuildHashTableFromCells::new(
            context,
            build_hash_table_from_cells::Settings { workgroup_size },
        );
        let allocate_blocks = AllocateBlocks::new(
            context,
            allocate_blocks::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );

        Self {
            find_cell_boundaries,
            prefix_sum,
            build_cells,
            color_cells,
            build_hash_table_from_cells,
            allocate_blocks,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_particles,
            positions,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let find_cell_boundaries::Output { boundaries } = self.find_cell_boundaries.record(
            context,
            encoder,
            find_cell_boundaries::Input {
                indirect: indirect_particles.clone(),
                positions: positions.clone(),
            },
            find_cell_boundaries::Parameters,
        )?;

        let prefix_sum::Output {
            prefix_sums: prefixed_boundaries,
        } = self.prefix_sum.record(
            context,
            encoder,
            prefix_sum::Input {
                indirect: indirect_particles.clone(),
                numbers: boundaries,
            },
            prefix_sum::Parameters,
        )?;

        let build_cells::Output {
            cell_ids,
            index_ranges: cell_index_ranges,
            new_indirect: indirect_cells,
            new_indirect_batch: indirect_cells_batch,
        } = self.build_cells.record(
            context,
            encoder,
            build_cells::Input {
                indirect: indirect_particles.clone(),
                positions,
                prefixed_boundaries,
            },
            build_cells::Parameters,
        )?;

        let color_cells::Output {
            indirect_colors,
            indirect_colors_batch,
            indices: cell_indices,
        } = self.color_cells.record(
            context,
            encoder,
            color_cells::Input {
                indirect: indirect_cells.clone(),
                cell_ids: cell_ids.clone(),
            },
            color_cells::Parameters,
        )?;

        let build_hash_table_from_cells::Output {
            block_table,
            owns: cell_owns,
        } = self.build_hash_table_from_cells.record(
            context,
            encoder,
            build_hash_table_from_cells::Input {
                indirect: indirect_cells.clone(),
                cell_ids: cell_ids.clone(),
            },
            build_hash_table_from_cells::Parameters,
        )?;

        let allocate_blocks::Output { block_offsets } = self.allocate_blocks.record(
            context,
            encoder,
            allocate_blocks::Input {
                indirect: indirect_cells.clone(),
                owns: cell_owns.clone(),
            },
            allocate_blocks::Parameters,
        )?;

        Ok(Output {
            indirect_cells,
            indirect_cells_batch,
            indirect_colors,
            indirect_colors_batch,
            cell_indices,
            cell_index_ranges,
            cell_ids,
            cell_owns,
            block_offsets,
            block_table,
        })
    }
}
