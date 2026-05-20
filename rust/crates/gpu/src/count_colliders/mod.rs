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

pub struct CountColliders {
    count_colliders: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub cell_size: f32,
}

pub struct Parameters;

pub struct Input {
    pub vertices: Allocation,
    pub triangles: Allocation,

    pub cell_ids: Allocation,
    pub cell_owns: Allocation,
    pub block_offsets: Allocation,
    pub block_table: Allocation,
}

#[derive(Clone)]
pub struct InputData<'a> {
    pub positions: &'a [Vector4<f32>],
    pub vertices: &'a [Vector4<f32>],
    pub triangles: &'a [Vector4<u32>],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            cell_size,
        }: Settings,
        InputData {
            positions,
            vertices,
            triangles,
        }: InputData,
    ) -> Self {
        assert!(triangles.iter().all(|triangle| {
            triangle.x < vertices.len() as u32
                && triangle.y < vertices.len() as u32
                && triangle.z < vertices.len() as u32
        }));

        let permutation = sort_positions_into_cells_on_cpu(
            &(0..positions.len() as u32).collect::<Vec<_>>(),
            positions,
            cell_size,
        );
        let permutation = permutation.as_slice();
        let positions = permutation.permute(positions);

        let boundaries = find_cell_boundaries_on_cpu(&positions, cell_size);
        let prefixed_boundaries = prefix_sum_on_cpu(&boundaries);
        let (cell_ids, _, _) = build_cells_on_cpu(
            workgroup_size,
            (u16::MAX as u32).try_into().unwrap(),
            cell_size,
            &positions,
            &prefixed_boundaries,
        );

        let (block_table, owns) = build_hash_table_on_cpu(&cell_ids);
        let pops: Vec<_> = owns.iter().cloned().map(u32::count_ones).collect();
        let block_offsets = prefix_sum_on_cpu(&pops);

        let vertices = Allocation::new(device, "vertices", vertices);
        let triangles = Allocation::new(device, "triangles", triangles);

        let cell_ids = Allocation::new(device, "cell_ids", &cell_ids);
        let cell_owns = Allocation::new(device, "cell_owns", &owns);
        let block_offsets = Allocation::new(device, "block_offsets", &block_offsets);
        let block_table = Allocation::new(device, "block_table", &block_table);

        Self {
            vertices,
            triangles,
            cell_ids,
            cell_owns,
            block_offsets,
            block_table,
        }
    }
}

pub struct Output {
    pub collider_counts: Allocation,
}

impl PipelinePart for CountColliders {
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
                    (Indirect::MIN_BINDING_SIZE, true),        // indirect_colors_batch
                    (u32::MIN_BINDING_SIZE, false),            // indices
                    (Vector4::<i32>::MIN_BINDING_SIZE, false), // cells
                    (u32::MIN_BINDING_SIZE, false),            // index_ranges
                    (u32::MIN_BINDING_SIZE, false),            // owns
                    (u32::MIN_BINDING_SIZE, false),            // block_table
                    (u32::MIN_BINDING_SIZE, false),            // block_offsets
                    (f32::MIN_BINDING_SIZE, false),            // masses
                    (f32::MIN_BINDING_SIZE, false),            // initial_volumes
                    (particle_parameters::Device::MIN_BINDING_SIZE, false), // parameters
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // positions
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false), // position_gradients
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // velocities
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false), // velocity_gradients
                    (Block::MIN_BINDING_SIZE, false),          // blocks
                ],
                immediate_size: 4,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("CELL_SIZE", cell_size as f64),
                    ("TIME_STEP", time_step as f64),
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
            indirect_colors_batch,
            cell_indices,
            cell_index_ranges,
            cell_ids,
            cell_owns,
            block_offsets,
            block_table,
            masses,
            initial_volumes,
            particle_parameters,
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let blocks = context.allocator()?.allocate::<Block>(
            "blocks",
            (cell_indices.len::<u32>().get()).try_into().unwrap(),
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
                    indirect_colors_batch.binding(),
                    cell_indices.binding(),
                    cell_ids.binding(),
                    cell_index_ranges.binding(),
                    cell_owns.binding(),
                    block_table.binding(),
                    block_offsets.binding(),
                    masses.binding(),
                    initial_volumes.binding(),
                    particle_parameters.binding(),
                    positions.binding(),
                    position_gradients.binding(),
                    velocities.binding(),
                    velocity_gradients.binding(),
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
