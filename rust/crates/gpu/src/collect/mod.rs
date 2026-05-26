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

use nalgebra::{Matrix4x3, Vector4};

use super::*;

pub struct Collect {
    collect: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub cell_size: f32,
    pub time_step: f32,
}

pub struct Parameters;

pub struct Input {
    pub indirect_cells_batch: Allocation,

    pub cell_index_ranges: Allocation,
    pub cell_ids: Allocation,

    pub block_ids: Allocation,
    pub block_table: Allocation,

    pub positions: Allocation,
    pub position_gradients: Allocation,
    pub velocities: Allocation,
    pub velocity_gradients: Allocation,

    pub blocks: Allocation,
}

#[derive(Debug)]
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
            time_step,
        }: Settings,
        dispatch_limit: NonZeroU32,
        subgroup_size: NonZeroU32,
        input_data @ scatter::InputData {
            masses,
            initial_volumes,
            particle_parameters,
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
        }: scatter::InputData,
    ) -> Self {
        assert_eq!(masses.len(), initial_volumes.len());
        assert_eq!(masses.len(), particle_parameters.len());
        assert_eq!(masses.len(), positions.len());
        assert_eq!(masses.len(), position_gradients.len());
        assert_eq!(masses.len(), velocities.len());
        assert_eq!(masses.len(), velocity_gradients.len());

        let grid_cpu = scatter_on_cpu(cell_size, time_step, input_data);

        let indices = sort_positions_into_cells_on_cpu(
            &(0..positions.len() as u32).collect::<Vec<_>>(),
            positions,
            cell_size,
        );

        fn permute<T: Clone>(indices: &[u32], to_permute: &[T]) -> Vec<T> {
            indices
                .iter()
                .map(|&index| to_permute[index as usize].clone())
                .collect()
        }

        let positions = permute(&indices, positions);
        let position_gradients = permute(&indices, position_gradients);
        let velocities = permute(&indices, velocities);
        let velocity_gradients = permute(&indices, velocity_gradients);

        let boundaries = find_cell_boundaries_on_cpu(&positions, cell_size);
        let prefixed_boundaries = prefix_sum_on_cpu(&boundaries);
        let (cell_ids, index_ranges, indirect_cells) = build_cells_on_cpu(
            workgroup_size,
            dispatch_limit,
            cell_size,
            &positions,
            &prefixed_boundaries,
        );
        let mut indirect_cells_batch = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: indirect_cells.len * subgroup_size.get(),
        });
        indirect_cells_batch.len = indirect_cells.len;

        let (block_ids, block_table) = build_hash_table_on_cpu_simple(&cell_ids);

        let blocks = {
            let nodes = gpu_grid_to_cpu_grid(&block_ids)
                .iter()
                .map(|cell_id| *grid_cpu.get(&cell_id.xyz()).unwrap_or(&Vector4::zeros()))
                .collect::<Vec<_>>();
            assert!(nodes.len().is_multiple_of(8));
            nodes
                .chunks(8)
                .map(|chunk| Block {
                    nodes: chunk.try_into().unwrap(),
                })
                .collect::<Vec<_>>()
        };

        let indirect_cells_batch =
            Allocation::new(device, "indirect_cells_batch", &[indirect_cells_batch]);

        let cell_index_ranges = Allocation::new(device, "cell_index_ranges", &index_ranges);
        let cell_ids = Allocation::new(device, "cell_ids", &cell_ids);
        let block_ids = Allocation::new(device, "block_ids", &block_ids);
        let block_table = Allocation::new(device, "block_table", &block_table);

        let positions = Allocation::new(device, "positions", &positions);
        let position_gradients = Allocation::new(device, "position_gradients", &position_gradients);
        let velocities = Allocation::new(device, "velocities", &velocities);
        let velocity_gradients = Allocation::new(device, "velocity_gradients", &velocity_gradients);

        let blocks = Allocation::new(device, "blocks", &blocks);

        Self {
            indirect_cells_batch,
            cell_index_ranges,
            cell_ids,
            block_ids,
            block_table,
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
            blocks,
        }
    }
}

pub struct Output {
    pub positions: Allocation,
    pub position_gradients: Allocation,
    pub velocities: Allocation,
    pub velocity_gradients: Allocation,
}

impl PipelinePart for Collect {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            cell_size,
            time_step,
        }: Settings,
    ) -> Self {
        let_compiled_module!(
            collect,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),        // indirect_cells_batch
                    (Vector4::<i32>::MIN_BINDING_SIZE, false), // cell_ids
                    (u32::MIN_BINDING_SIZE, false),            // index_ranges
                    (Vector4::<i32>::MIN_BINDING_SIZE, false), // block_ids
                    (u32::MIN_BINDING_SIZE, false),            // block_table
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // positions
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false), // position_gradients
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // velocities
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false), // velocity_gradients
                    (Block::MIN_BINDING_SIZE, false),          // blocks
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("CELL_SIZE", cell_size as f64),
                    ("TIME_STEP", time_step as f64),
                ]
            }
        );

        Self { collect }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_cells_batch,
            cell_index_ranges,
            cell_ids,
            block_ids,
            block_table,
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
            blocks,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let mut compute_pass = encoder.begin_compute_pass(self.collect.label);
        compute_pass.set_pipeline(&self.collect.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.collect,
                [
                    indirect_cells_batch.binding(),
                    cell_ids.binding(),
                    cell_index_ranges.binding(),
                    block_ids.binding(),
                    block_table.binding(),
                    positions.binding(),
                    position_gradients.binding(),
                    velocities.binding(),
                    velocity_gradients.binding(),
                    blocks.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(
            indirect_cells_batch.buffer(),
            indirect_cells_batch.offset(),
        );

        Ok(Output {
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
        })
    }
}
