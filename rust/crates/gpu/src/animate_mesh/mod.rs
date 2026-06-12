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
use rustc_hash::FxHashMap;
use squishy_volumes_util::triangle::Triangle;

use super::*;

pub struct AnimateMesh {
    move_vertices: CompiledModule,
    compute_triangle_normals: CompiledModule,
    compute_vertex_normals: CompiledModule,

    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

pub struct Parameters {
    pub factor: f32,
}

pub struct Input {
    pub vertex_positions_start: Allocation,
    pub vertex_positions_end: Allocation,
    pub vertex_triangle_offsets: Allocation,
    pub vertex_triangle_lists: Allocation,
    pub triangle_indices: Allocation,
}

#[derive(Clone)]
pub struct InputData<'a> {
    pub vertex_positions_start: &'a [Vector4<f32>],
    pub vertex_positions_end: &'a [Vector4<f32>],
    pub triangle_indices: &'a [Triangle],
}

fn triangle_lists(num_vertices: usize, triangle_indices: &[Triangle]) -> Vec<Vec<u32>> {
    let mut vertex_to_triangles: Vec<Vec<u32>> = vec![Default::default(); num_vertices];
    for (triangle_index, indices) in triangle_indices.iter().enumerate() {
        for vertex_index in indices.iter() {
            vertex_to_triangles[*vertex_index as usize].push(triangle_index as u32);
        }
    }
    vertex_to_triangles
        .iter_mut()
        .enumerate()
        .for_each(|(this_vertex, triangles)| {
            let mut neighbor_counts: FxHashMap<u32, u8> = Default::default();
            for triangle_index in triangles.iter() {
                for &vertex_index in triangle_indices[*triangle_index as usize].iter() {
                    if vertex_index != this_vertex as u32 {
                        *neighbor_counts.entry(vertex_index).or_default() += 1;
                    }
                }
            }
            assert!(neighbor_counts.values().all(|&count| count <= 2));
            if neighbor_counts.into_values().any(|count| count != 2) {
                triangles.clear();
            }
        });
    vertex_to_triangles.push(Default::default());
    vertex_to_triangles
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        InputData {
            vertex_positions_start,
            vertex_positions_end,
            triangle_indices,
        }: InputData,
    ) -> Self {
        assert_eq!(vertex_positions_start.len(), vertex_positions_end.len());
        assert!(triangle_indices.iter().all(|indices| {
            indices
                .iter()
                .all(|&index| (index as usize) < vertex_positions_start.len())
        }));

        let vertex_triangle_lists = triangle_lists(vertex_positions_start.len(), triangle_indices);

        let vertex_triangle_offsets = prefix_sum_on_cpu(
            &vertex_triangle_lists
                .iter()
                .map(|v| v.len() as u32)
                .collect::<Vec<_>>(),
        );
        let mut vertex_triangle_lists = vertex_triangle_lists
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        if vertex_triangle_lists.is_empty() {
            vertex_triangle_lists.push(0);
        }

        let vertex_positions_start =
            Allocation::new(device, "vertex_positions_start", vertex_positions_start);
        let vertex_positions_end =
            Allocation::new(device, "vertex_positions_end", vertex_positions_end);
        let vertex_triangle_offsets =
            Allocation::new(device, "vertex_triangle_offsets", &vertex_triangle_offsets);
        let vertex_triangle_lists =
            Allocation::new(device, "vertex_triangle_lists", &vertex_triangle_lists);

        let triangle_indices = Allocation::new(device, "triangle_indices", triangle_indices);

        Self {
            vertex_positions_start,
            vertex_positions_end,
            vertex_triangle_offsets,
            vertex_triangle_lists,
            triangle_indices,
        }
    }
}

pub struct Output {
    pub vertex_positions: Allocation,
    pub vertex_normals: Allocation,
    pub triangle_normals: Allocation,
}

impl PipelinePart for AnimateMesh {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
    ) -> Self {
        let_compiled_module!(
            move_vertices,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // vertex_positions_start
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // vertex_positions_end
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // vertex_positions
                ],
                immediate_size: 4,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64),]
            }
        );

        let_compiled_module!(
            compute_triangle_normals,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // vertex_positions_start
                    (Triangle::MIN_BINDING_SIZE, false),       // triangle_indices
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // triangle_normals
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64),]
            }
        );

        let_compiled_module!(
            compute_vertex_normals,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (u32::MIN_BINDING_SIZE, false),            // vertex_triangle_offsets
                    (u32::MIN_BINDING_SIZE, false),            // vertex_triangle_lists
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // vertex_normals
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // triangle_normals
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64),]
            }
        );

        Self {
            move_vertices,
            compute_triangle_normals,
            compute_vertex_normals,
            workgroup_size,
            dispatch_limit,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            vertex_positions_start,
            vertex_positions_end,
            vertex_triangle_offsets,
            vertex_triangle_lists,
            triangle_indices,
        }: Input,
        Parameters { factor }: Parameters,
    ) -> Result<Output, GpuError> {
        let num_vertices = vertex_positions_start.len::<Vector4<f32>>();
        let num_triangles = triangle_indices.len::<Triangle>();
        let vertex_positions = context
            .allocator()?
            .allocate::<Vector4<f32>>("vertex_positions", num_vertices)?;
        let triangle_normals = context
            .allocator()?
            .allocate::<Vector4<f32>>("triangle_normals", num_triangles)?;
        let vertex_normals = context
            .allocator()?
            .allocate::<Vector4<f32>>("vertex_normals", num_vertices)?;

        {
            let mut compute_pass = encoder.begin_compute_pass(self.move_vertices.label);
            compute_pass.set_pipeline(&self.move_vertices.compute_pipeline);
            compute_pass.set_bind_group(
                0,
                &create_bind_group(
                    context.device(),
                    &self.move_vertices,
                    [
                        vertex_positions_start.binding(),
                        vertex_positions_end.binding(),
                        vertex_positions.binding(),
                    ],
                ),
                &[],
            );
            let Indirect { x, y, z, .. } = Indirect::new(IndirectSettings {
                workgroup_size: self.workgroup_size,
                dispatch_limit: self.dispatch_limit,
                len: num_vertices.get() as u32,
            });
            compute_pass.set_immediates(0, bytemuck::bytes_of(&factor));
            compute_pass.dispatch_workgroups(x, y, z);
        }

        {
            let mut compute_pass = encoder.begin_compute_pass(self.compute_triangle_normals.label);
            compute_pass.set_pipeline(&self.compute_triangle_normals.compute_pipeline);
            compute_pass.set_bind_group(
                0,
                &create_bind_group(
                    context.device(),
                    &self.compute_triangle_normals,
                    [
                        vertex_positions.binding(),
                        triangle_indices.binding(),
                        triangle_normals.binding(),
                    ],
                ),
                &[],
            );
            let Indirect { x, y, z, .. } = Indirect::new(IndirectSettings {
                workgroup_size: self.workgroup_size,
                dispatch_limit: self.dispatch_limit,
                len: num_triangles.get() as u32,
            });
            compute_pass.dispatch_workgroups(x, y, z);
        }

        {
            let mut compute_pass = encoder.begin_compute_pass(self.compute_vertex_normals.label);
            compute_pass.set_pipeline(&self.compute_vertex_normals.compute_pipeline);
            compute_pass.set_bind_group(
                0,
                &create_bind_group(
                    context.device(),
                    &self.compute_vertex_normals,
                    [
                        vertex_triangle_offsets.binding(),
                        vertex_triangle_lists.binding(),
                        vertex_normals.binding(),
                        triangle_normals.binding(),
                    ],
                ),
                &[],
            );
            let Indirect { x, y, z, .. } = Indirect::new(IndirectSettings {
                workgroup_size: self.workgroup_size,
                dispatch_limit: self.dispatch_limit,
                len: num_vertices.get() as u32,
            });
            compute_pass.dispatch_workgroups(x, y, z);
        }

        Ok(Output {
            vertex_positions,
            vertex_normals,
            triangle_normals,
        })
    }
}
