// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use std::num::NonZeroU32;

use nalgebra::Vector4;

use super::*;

pub struct Scatter {
    scatter: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub cell_size: f32,
}

pub struct Parameters;

pub struct Input {
    pub indirect_particles: Allocation,
    pub indirect_colors_batch: Allocation,

    pub cell_indices: Allocation,
    pub cell_index_ranges: Allocation,
    pub cell_ids: Allocation,
    pub cell_owns: Allocation,
    pub block_offsets: Allocation,
    pub block_table: Allocation,
    pub positions: Allocation,
}

pub struct InputAddendum {
    pub indirect_colors_batch: Vec<Indirect>,
    pub cell_ids: Vec<Vector4<i32>>,
    pub cell_owns: Vec<u32>,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            cell_size,
        }: Settings,
        dispatch_limit: NonZeroU32,
        subgroup_size: NonZeroU32,
        positions: &[Vector4<f32>],
    ) -> (Self, InputAddendum) {
        let indirect_particles = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: positions.len() as u32,
        });

        let positions: Vec<Vector4<f32>> = sort_positions_into_cells_on_cpu(
            &(0..positions.len() as u32).collect::<Vec<_>>(),
            positions,
            cell_size,
        )
        .into_iter()
        .map(|index| positions[index as usize])
        .collect();

        let boundaries = find_cell_boundaries_on_cpu(&positions, cell_size);
        let prefixed_boundaries = prefix_sum_on_cpu(&boundaries);
        let (cell_ids, index_ranges, _indirect_cells) = build_cells_on_cpu(
            workgroup_size,
            dispatch_limit,
            cell_size,
            &positions,
            &prefixed_boundaries,
        );

        let (block_table, owns) = build_hash_table_on_cpu(&cell_ids);
        let pops: Vec<_> = owns.iter().cloned().map(u32::count_ones).collect();
        let block_offsets = prefix_sum_on_cpu(&pops);

        let (_indirect_colors, indirect_colors_batch, cell_indices) =
            color_cells_on_cpu(workgroup_size, dispatch_limit, subgroup_size, &cell_ids);

        let addendum = InputAddendum {
            indirect_colors_batch: indirect_colors_batch.clone(),
            cell_ids: cell_ids.clone(),
            cell_owns: owns.clone(),
        };

        let indirect_particles =
            Allocation::new(device, "indirect_particles", &[indirect_particles]);
        let indirect_colors_batch =
            Allocation::new(device, "indirect_colors_batch", &indirect_colors_batch);

        let cell_indices = Allocation::new(device, "cell_indices", &cell_indices);
        let cell_index_ranges = Allocation::new(device, "cell_index_ranges", &index_ranges);
        let cell_ids = Allocation::new(device, "cell_ids", &cell_ids);
        let cell_owns = Allocation::new(device, "cell_owns", &owns);
        let block_offsets = Allocation::new(device, "block_offsets", &block_offsets);
        let block_table = Allocation::new(device, "block_table", &block_table);
        let positions = Allocation::new(device, "positions", &positions);

        (
            Self {
                indirect_particles,
                indirect_colors_batch,
                cell_indices,
                cell_index_ranges,
                cell_ids,
                cell_owns,
                block_offsets,
                block_table,
                positions,
            },
            addendum,
        )
    }
}

pub struct Output {
    pub blocks: Allocation,
}

impl PipelinePart for Scatter {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            cell_size,
        }: Settings,
    ) -> Self {
        let_compiled_module!(
            scatter,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                    (Vector4::<i32>::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),
                    (Block::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 4,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("CELL_SIZE", cell_size as f64),
                ]
            }
        );

        Self { scatter }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_particles,
            indirect_colors_batch,
            cell_indices,
            cell_index_ranges,
            cell_ids,
            cell_owns,
            block_offsets,
            block_table,
            positions,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let blocks = context.allocator()?.allocate::<Block>(
            "blocks",
            (cell_indices.len::<u32>().get() * 8).try_into().unwrap(),
        )?;

        encoder.clear_buffer(blocks.buffer(), blocks.offset(), Some(blocks.size().get()));

        let mut compute_pass = encoder.begin_compute_pass(self.scatter.label);
        compute_pass.set_pipeline(&self.scatter.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.scatter,
                [
                    indirect_particles.binding(),
                    indirect_colors_batch.binding(),
                    cell_indices.binding(),
                    cell_ids.binding(),
                    cell_index_ranges.binding(),
                    cell_owns.binding(),
                    block_table.binding(),
                    block_offsets.binding(),
                    positions.binding(),
                    blocks.binding(),
                ],
            ),
            &[],
        );
        for color in 0..8u32 {
            compute_pass.set_immediates(0, bytemuck::bytes_of(&color));
            compute_pass.dispatch_workgroups_indirect(
                indirect_colors_batch.buffer(),
                indirect_colors_batch.offset() + Indirect::MIN_BINDING_SIZE.get() * color as u64,
            );
        }

        Ok(Output { blocks })
    }
}
