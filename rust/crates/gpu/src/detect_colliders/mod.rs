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

pub struct DetectColliders {
    detect_colliders: CompiledModule,

    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub cell_size: f32,
    pub layers: u32,
}

pub struct Parameters;

pub struct AllocatedMesh {
    pub vertices: Allocation,
    pub triangles: Allocation,
}

pub struct Input {
    pub collider_meshes: Vec<AllocatedMesh>,
    pub block_ids: Allocation,
    pub block_table: Allocation,
}

#[derive(Clone)]
pub struct InputData<'a> {
    pub collider_meshes: Vec<(&'a [Vector4<f32>], &'a [Triangle])>,
    pub block_ids: &'a [Vector4<i32>],
    pub block_table: &'a [u32],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        InputData {
            collider_meshes,
            block_ids,
            block_table,
        }: InputData,
    ) -> Self {
        for (vertices, triangles) in &collider_meshes {
            assert!(
                triangles
                    .iter()
                    .all(|Triangle { a, b, c }| (*a as usize) < vertices.len()
                        && (*b as usize) < vertices.len()
                        && (*c as usize) < vertices.len())
            );
        }

        let collider_meshes = collider_meshes
            .into_iter()
            .map(|(vertices, triangles)| {
                let vertices = Allocation::new(device, "vertices", vertices);
                let triangles = Allocation::new(device, "triangles", triangles);
                AllocatedMesh {
                    vertices,
                    triangles,
                }
            })
            .collect();

        let block_ids = Allocation::new(device, "block_ids", block_ids);
        let block_table = Allocation::new(device, "block_table", block_table);

        Self {
            collider_meshes,
            block_ids,
            block_table,
        }
    }
}

pub struct Output {
    pub collider_bits: Allocation,
}

impl PipelinePart for DetectColliders {
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
            layers,
        }: Settings,
    ) -> Self {
        let_compiled_module!(
            detect_colliders,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // vertices
                    (Triangle::MIN_BINDING_SIZE, false),       // triangles
                    (Vector4::<i32>::MIN_BINDING_SIZE, false), // block_ids
                    (u32::MIN_BINDING_SIZE, false),            // block_table
                    (u32::MIN_BINDING_SIZE, false),            // collider_bits
                ],
                immediate_size: 4,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("CELL_SIZE", cell_size as f64),
                    ("LAYERS", layers as f64),
                ]
            }
        );

        Self {
            detect_colliders,
            workgroup_size,
            dispatch_limit,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            collider_meshes,
            block_ids,
            block_table,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let collider_bits = context
            .allocator()?
            .allocate::<u32>("collider_bits", block_ids.len::<Vector4<i32>>())?;
        encoder.clear_buffer(
            collider_bits.buffer(),
            collider_bits.offset(),
            Some(collider_bits.size().get()),
        );

        {
            let mut compute_pass = encoder.begin_compute_pass(self.detect_colliders.label);
            compute_pass.set_pipeline(&self.detect_colliders.compute_pipeline);
            for (
                collider_index,
                AllocatedMesh {
                    vertices,
                    triangles,
                },
            ) in collider_meshes.into_iter().enumerate()
            {
                compute_pass.set_bind_group(
                    0,
                    &create_bind_group(
                        context.device(),
                        &self.detect_colliders,
                        [
                            vertices.binding(),
                            triangles.binding(),
                            block_ids.binding(),
                            block_table.binding(),
                            collider_bits.binding(),
                        ],
                    ),
                    &[],
                );
                let Indirect { x, y, z, .. } = Indirect::new(IndirectSettings {
                    workgroup_size: self.workgroup_size,
                    dispatch_limit: self.dispatch_limit,
                    len: triangles.len::<Triangle>().get() as u32 * context.subgroup_size().get(),
                });
                compute_pass.set_immediates(0, bytemuck::bytes_of(&(collider_index as u32)));
                compute_pass.dispatch_workgroups(x, y, z);
            }
        }

        Ok(Output { collider_bits })
    }
}
