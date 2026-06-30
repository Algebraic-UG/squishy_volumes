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
use squishy_volumes_util::{mesh::compute_triangle_lists, triangle::Triangle};

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

impl Input {
    pub fn new(
        device: &wgpu::Device,
        InputData {
            vertex_positions_start,
            vertex_positions_end,
            triangle_indices,
        }: InputData,
    ) -> Result<Self, GpuAllocatorError> {
        assert_eq!(vertex_positions_start.len(), vertex_positions_end.len());
        assert!(triangle_indices.iter().all(|indices| {
            indices
                .iter()
                .all(|&index| (index as usize) < vertex_positions_start.len())
        }));

        let vertex_triangle_lists =
            compute_triangle_lists(vertex_positions_start.len(), triangle_indices);

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
            Allocation::new(device, "vertex_positions_start", vertex_positions_start)?;
        let vertex_positions_end =
            Allocation::new(device, "vertex_positions_end", vertex_positions_end)?;
        let vertex_triangle_offsets =
            Allocation::new(device, "vertex_triangle_offsets", &vertex_triangle_offsets)?;
        let vertex_triangle_lists =
            Allocation::new(device, "vertex_triangle_lists", &vertex_triangle_lists)?;

        let triangle_indices = Allocation::new(device, "triangle_indices", triangle_indices)?;

        Ok(Self {
            vertex_positions_start,
            vertex_positions_end,
            vertex_triangle_offsets,
            vertex_triangle_lists,
            triangle_indices,
        })
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
            let [x, y, z] = Indirect::new(DispatchSettings {
                workgroup_size: self.workgroup_size,
                dispatch_limit: self.dispatch_limit,
                len: num_vertices.get() as u32,
            })
            .direct();
            let mut compute_pass = context.enter_module(
                encoder,
                &self.move_vertices,
                [
                    vertex_positions_start.binding(),
                    vertex_positions_end.binding(),
                    vertex_positions.binding(),
                ],
            );
            compute_pass.set_immediates(0, bytemuck::bytes_of(&factor));
            compute_pass.dispatch_workgroups(x, y, z);
        }

        {
            let [x, y, z] = Indirect::new(DispatchSettings {
                workgroup_size: self.workgroup_size,
                dispatch_limit: self.dispatch_limit,
                len: num_triangles.get() as u32,
            })
            .direct();

            context
                .enter_module(
                    encoder,
                    &self.compute_triangle_normals,
                    [
                        vertex_positions.binding(),
                        triangle_indices.binding(),
                        triangle_normals.binding(),
                    ],
                )
                .dispatch_workgroups(x, y, z);
        }

        {
            let [x, y, z] = Indirect::new(DispatchSettings {
                workgroup_size: self.workgroup_size,
                dispatch_limit: self.dispatch_limit,
                len: num_vertices.get() as u32,
            })
            .direct();

            context
                .enter_module(
                    encoder,
                    &self.compute_vertex_normals,
                    [
                        vertex_triangle_offsets.binding(),
                        vertex_triangle_lists.binding(),
                        vertex_normals.binding(),
                        triangle_normals.binding(),
                    ],
                )
                .dispatch_workgroups(x, y, z);
        }

        Ok(Output {
            vertex_positions,
            vertex_normals,
            triangle_normals,
        })
    }
}
