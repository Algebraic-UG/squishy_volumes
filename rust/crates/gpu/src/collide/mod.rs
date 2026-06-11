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
use squishy_volumes_util::{
    BoundingVolumeHierarchy, bounding_volume_hierarchy::triangles_to_leaf_aabbs, triangle::Triangle,
};

use super::*;

pub struct Collide {
    workgroup_size: NonZeroU32,
    collide: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub positions: Allocation,
    pub velocities: Allocation,
    pub vertices: Allocation,
    pub triangles: Allocation,
    pub bvh: BoundingVolumeHierarchyAllocations,
}

pub struct InputData<'a> {
    pub margin: f32,
    pub leaf_size: f32,
    pub leaf_threshold: u32,
    pub positions: &'a [Vector4<f32>],
    pub velocities: &'a [Vector4<f32>],
    pub vertices: &'a [Vector4<f32>],
    pub triangles: &'a [Triangle],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        InputData {
            margin,
            leaf_size,
            leaf_threshold,
            positions,
            velocities,
            vertices,
            triangles,
        }: InputData,
    ) -> Self {
        assert!(triangles.iter().all(|triangle| {
            triangle
                .iter()
                .all(|&index| (index as usize) < vertices.len())
        }));

        let vertices_3d: Vec<_> = vertices.iter().map(Vector4::xyz).collect();
        let aabbs = triangles_to_leaf_aabbs(leaf_size, margin, &vertices_3d, triangles);

        let bvh = BoundingVolumeHierarchy::new(aabbs, leaf_threshold).unwrap();

        let positions = Allocation::new(device, "positions", positions);
        let velocities = Allocation::new(device, "velocities", velocities);
        let vertices = Allocation::new(device, "vertice", vertices);
        let triangles = Allocation::new(device, "triangles", triangles);

        let bvh = BoundingVolumeHierarchyAllocations::new(device, leaf_size, &bvh);

        Self {
            positions,
            velocities,
            vertices,
            triangles,
            bvh,
        }
    }
}

pub struct Output {
    pub positions: Allocation,
    pub velocities: Allocation,
}

impl PipelinePart for Collide {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &GpuContext, Settings { workgroup_size }: Settings) -> Self {
        let_compiled_module!(
            collide,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // positions
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // velocites
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // vertices
                    (Triangle::MIN_BINDING_SIZE, false),       // triangles
                    (BoundingVolumeHierarchyMeta::MIN_BINDING_SIZE, false), // bvh_meta
                    (u32::MIN_BINDING_SIZE, false),            // bvh_nodes
                    (u32::MIN_BINDING_SIZE, false),            // bvh_indices
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64),]
            }
        );

        Self {
            collide,
            workgroup_size,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            positions,
            velocities,
            vertices,
            triangles,
            bvh,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let mut compute_pass = encoder.begin_compute_pass(self.collide.label);
        compute_pass.set_pipeline(&self.collide.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.collide,
                [
                    positions.binding(),
                    velocities.binding(),
                    vertices.binding(),
                    triangles.binding(),
                    bvh.meta.binding(),
                    bvh.nodes.binding(),
                    bvh.indices.binding(),
                ],
            ),
            &[],
        );
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            len: positions.len::<Vector4<f32>>().get() as u32,
        });
        compute_pass.dispatch_workgroups(indirect.x, indirect.y, indirect.z);

        Ok(Output {
            positions,
            velocities,
        })
    }
}
