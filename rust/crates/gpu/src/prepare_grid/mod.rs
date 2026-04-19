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
    sort_positions_into_cells: SortPositionsIntoCells,
    permute_particles: PermuteParticles,
    find_cell_boundaries: FindCellBoundaries,
    prefix_sum: PrefixSum,
    build_cells: BuildCells,
    color_cells: ColorCells,
    build_hash_table: BuildHashTable,
    allocate_blocks: AllocateBlocks,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub bit_count: NonZeroU32,
    pub cell_size: f32,
}

pub struct Parameters;

pub struct Input {
    pub particle_indirect: Allocation,
    pub indices_in: Allocation,
    pub positions_in: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
            ..
        }: Settings,
        positions: &[Vector4<f32>],
    ) -> Self {
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: positions.len() as u32,
        });

        let particle_indirect = Allocation::new(device, "particle_indirect", &[indirect]);
        let indices_in = Allocation::new(
            device,
            "indices_in",
            &(0..positions.len() as u32).collect::<Vec<_>>(),
        );
        let positions_in = Allocation::new(device, "positions_in", positions);

        Self {
            particle_indirect,
            indices_in,
            positions_in,
        }
    }
}

pub struct Output {
    pub indirect_cells: Allocation,
    pub indirect_colors: Allocation,
    pub indirect_colors_batch: Allocation,

    pub indices_out: Allocation,
    pub positions_out: Allocation,
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
            bit_count,
            cell_size,
        }: Settings,
    ) -> Self {
        let sort_positions_into_cells = SortPositionsIntoCells::new(
            context,
            sort_positions_into_cells::Settings {
                workgroup_size,
                dispatch_limit,
                cell_size,
                bit_count,
            },
        );
        let permute_particles =
            PermuteParticles::new(context, permute_particles::Settings { workgroup_size });
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
        let build_hash_table =
            BuildHashTable::new(context, build_hash_table::Settings { workgroup_size });
        let allocate_blocks = AllocateBlocks::new(
            context,
            allocate_blocks::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );

        Self {
            sort_positions_into_cells,
            permute_particles,
            find_cell_boundaries,
            prefix_sum,
            build_cells,
            color_cells,
            build_hash_table,
            allocate_blocks,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            particle_indirect,
            indices_in,
            positions_in,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let sort_positions_into_cells::Output {
            indices_out: permutation,
        } = self.sort_positions_into_cells.record(
            context,
            encoder,
            sort_positions_into_cells::Input {
                indirect: particle_indirect.clone(),
                positions: positions_in.clone(),
            },
            sort_positions_into_cells::Parameters,
        )?;

        let permute_particles::Output {
            indices_out,
            positions_out,
        } = self.permute_particles.record(
            context,
            encoder,
            permute_particles::Input {
                indirect: particle_indirect.clone(),
                permutation,
                indices_in,
                positions_in,
            },
            permute_particles::Parameters,
        )?;

        let find_cell_boundaries::Output { boundaries } = self.find_cell_boundaries.record(
            context,
            encoder,
            find_cell_boundaries::Input {
                indirect: particle_indirect.clone(),
                positions: positions_out.clone(),
            },
            find_cell_boundaries::Parameters,
        )?;

        let prefix_sum::Output {
            prefix_sums: prefixed_boundaries,
        } = self.prefix_sum.record(
            context,
            encoder,
            prefix_sum::Input {
                indirect: particle_indirect.clone(),
                numbers: boundaries,
            },
            prefix_sum::Parameters,
        )?;

        let build_cells::Output {
            cell_ids,
            index_ranges: cell_index_ranges,
            new_indirect: indirect_cells,
        } = self.build_cells.record(
            context,
            encoder,
            build_cells::Input {
                indirect: particle_indirect.clone(),
                positions: positions_out.clone(),
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

        let build_hash_table::Output {
            block_table,
            owns: cell_owns,
        } = self.build_hash_table.record(
            context,
            encoder,
            build_hash_table::Input {
                indirect_colors: indirect_colors.clone(),
                indices: cell_indices.clone(),
                cells: cell_ids.clone(),
            },
            build_hash_table::Parameters,
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
            indirect_colors,
            indirect_colors_batch,
            indices_out,
            positions_out,
            cell_indices,
            cell_index_ranges,
            cell_ids,
            cell_owns,
            block_offsets,
            block_table,
        })
    }
}
