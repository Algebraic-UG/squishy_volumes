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
use rand::seq::IndexedRandom;

use super::*;

pub struct DistanceToColliders {
    distance_to_colliders: CompiledModule,
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

pub struct InputMesh {
    pub vertices: Allocation,
    pub normals: Allocation,
    pub velocities: Allocation,

    pub triangles: Allocation,
    pub opposite_vertices: Allocation,
    pub frictions: Allocation,
}

pub struct Input {
    pub collider_meshes: Vec<InputMesh>,
    pub block_ids: Allocation,
    pub block_table: Allocation,
    pub collider_bits: Allocation,
    pub collider_offsets: Allocation,
}

pub struct InputDataMesh<'a> {
    pub vertices: &'a [Vector4<f32>],
    pub normals: &'a [Vector4<f32>],
    pub velocities: &'a [Vector4<f32>],

    pub triangles: &'a [Triangle],
    pub opposite_vertices: &'a [Triangle],
    pub frictions: &'a [f32],
}

#[derive(Clone)]
pub struct InputData<'a> {
    pub collider_meshes: &'a [InputDataMesh<'a>],
    pub block_ids: &'a [Vector4<i32>],
    pub block_table: &'a [u32],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            cell_size, layers, ..
        }: Settings,
        InputData {
            collider_meshes,
            block_ids,
            block_table,
        }: InputData,
    ) -> Self {
        assert!(!collider_meshes.is_empty());

        for InputDataMesh {
            vertices,
            normals,
            velocities,
            triangles,
            opposite_vertices,
            frictions,
        } in collider_meshes.iter()
        {
            assert_eq!(vertices.len(), normals.len());
            assert_eq!(vertices.len(), velocities.len());
            assert_eq!(triangles.len(), opposite_vertices.len());
            assert_eq!(triangles.len(), frictions.len());
            assert!(triangles.iter().all(|Triangle { a, b, c }| {
                [
                    (*a as usize) < vertices.len(),
                    (*b as usize) < vertices.len(),
                    (*c as usize) < vertices.len(),
                ]
                .into_iter()
                .all(|b| b)
            }));
            assert!(opposite_vertices.iter().all(|Triangle { a, b, c }| {
                [
                    *a != u32::MAX || (*a as usize) < vertices.len(),
                    *b != u32::MAX || (*b as usize) < vertices.len(),
                    *c != u32::MAX || (*c as usize) < vertices.len(),
                ]
                .into_iter()
                .all(|b| b)
            }));
        }

        let collider_bits = detect_colliders_on_cpu(
            cell_size,
            layers,
            &detect_colliders::InputData {
                collider_meshes: &collider_meshes
                    .iter()
                    .map(
                        |InputDataMesh {
                             vertices,
                             triangles,
                             ..
                         }| detect_colliders::InputDataMesh {
                            vertices,
                            triangles,
                        },
                    )
                    .collect::<Vec<_>>(),
                block_ids,
                block_table,
            },
        );
        let collider_pops: Vec<u32> = collider_bits.iter().map(|bits| bits.count_ones()).collect();

        let collider_offsets = prefix_sum_on_cpu(&collider_pops);

        let collider_bits = Allocation::new(device, "collider_bits", &collider_bits);
        let collider_offsets = Allocation::new(device, "collider_offsets", &collider_offsets);

        let collider_meshes = collider_meshes
            .into_iter()
            .map(
                |InputDataMesh {
                     vertices,
                     normals,
                     velocities,
                     triangles,
                     opposite_vertices,
                     frictions,
                 }| {
                    let vertices = Allocation::new(device, "vertices", vertices);
                    let normals = Allocation::new(device, "normals", normals);
                    let velocities = Allocation::new(device, "velocities", velocities);
                    let triangles = Allocation::new(device, "triangles", triangles);
                    let opposite_vertices =
                        Allocation::new(device, "opposite_vertices", opposite_vertices);
                    let frictions = Allocation::new(device, "frictions", frictions);
                    InputMesh {
                        vertices,
                        normals,
                        velocities,
                        triangles,
                        opposite_vertices,
                        frictions,
                    }
                },
            )
            .collect();

        let block_ids = Allocation::new(device, "block_ids", block_ids);
        let block_table = Allocation::new(device, "block_table", block_table);

        Self {
            collider_meshes,
            block_ids,
            block_table,
            collider_bits,
            collider_offsets,
        }
    }
}

pub struct Output {
    pub collider_normals_and_distances: Allocation,
    pub collider_velocities_and_frictions: Allocation,
}

impl PipelinePart for DistanceToColliders {
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
            distance_to_colliders,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // vertices
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // normals
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // velocities
                    (Triangle::MIN_BINDING_SIZE, false),       // triangles
                    (Triangle::MIN_BINDING_SIZE, false),       // opposite_vertices
                    (f32::MIN_BINDING_SIZE, false),            // friction
                    (Vector4::<i32>::MIN_BINDING_SIZE, false), // block_ids
                    (u32::MIN_BINDING_SIZE, false),            // block_table
                    (u32::MIN_BINDING_SIZE, false),            // collider_bits
                    (u32::MIN_BINDING_SIZE, false),            // collider_offsets
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // collider_normals_and_distances
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // collider_velocities_and_frictions
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
            distance_to_colliders,
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
            collider_bits,
            collider_offsets,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        assert!(!collider_meshes.is_empty());

        // TODO: this is overkill
        let collider_normals_and_distances = context.allocator()?.allocate::<Vector4<f32>>(
            "collider_normals_and_distances",
            (block_ids.len::<Vector4<i32>>().get() * collider_meshes.len() as u64)
                .try_into()
                .unwrap(),
        )?;
        let collider_velocities_and_frictions = context.allocator()?.allocate::<Vector4<f32>>(
            "collider_velocities_and_frictions",
            (block_ids.len::<Vector4<i32>>().get() * collider_meshes.len() as u64)
                .try_into()
                .unwrap(),
        )?;

        encoder.clear_buffer(
            collider_normals_and_distances.buffer(),
            collider_normals_and_distances.offset(),
            Some(collider_normals_and_distances.size().get()),
        );

        {
            let mut compute_pass = encoder.begin_compute_pass(self.count_colliders.label);
            compute_pass.set_pipeline(&self.count_colliders.compute_pipeline);
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
                        &self.count_colliders,
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

        let bits_to_pops::Output {
            pops: collider_pops,
        } = self.bits_to_pops.record(
            context,
            encoder,
            bits_to_pops::Input {
                indirect: indirect_blocks,
                bits: collider_bits.clone(),
            },
            bits_to_pops::Parameters,
        )?;

        Ok(Output {
            collider_bits,
            collider_pops,
        })
    }
}
