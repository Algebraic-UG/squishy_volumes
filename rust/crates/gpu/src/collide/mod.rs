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
    BoundingVolumeHierarchy,
    bounding_volume_hierarchy::triangles_to_leaf_aabbs,
    triangle::{Opposites, Triangle},
};

use super::*;

pub struct Collide {
    collide: CompiledModule,

    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub forget_distance: f32,
    pub accept_distance: f32,
    pub time_step: f32,
}

pub struct Parameters;

pub struct Input {
    pub particle_positions: Allocation,
    pub particle_collider_bits: Allocation,
    pub particle_velocities: Allocation,
    pub vertex_positions: Allocation,
    pub vertex_normals: Allocation,
    pub triangle_indices: Allocation,
    pub triangle_collider: Allocation,
    pub triangle_normals: Allocation,
    pub triangle_opposites: Allocation,
    pub triangle_frictions: Allocation,
    pub bvh: BoundingVolumeHierarchyAllocations,
}

pub struct InputData<'a> {
    pub leaf_size: f32,
    pub leaf_threshold: u32,
    pub particle_positions: &'a [Vector4<f32>],
    pub particle_collider_bits: &'a [u32],
    pub particle_velocities: &'a [Vector4<f32>],
    pub vertex_positions: &'a [Vector4<f32>],
    pub vertex_normals: &'a [Vector4<f32>],
    pub triangle_indices: &'a [Triangle],
    pub triangle_collider: &'a [u32],
    pub triangle_normals: &'a [Vector4<f32>],
    pub triangle_opposites: &'a [Opposites],
    pub triangle_frictions: &'a [f32],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            forget_distance, ..
        }: &Settings,
        InputData {
            leaf_size,
            leaf_threshold,
            particle_positions,
            particle_collider_bits,
            particle_velocities,
            vertex_positions,
            vertex_normals,
            triangle_indices,
            triangle_collider,
            triangle_normals,
            triangle_opposites,
            triangle_frictions,
        }: InputData,
    ) -> Self {
        assert_eq!(particle_positions.len(), particle_collider_bits.len());
        assert_eq!(particle_positions.len(), particle_velocities.len());
        assert_eq!(vertex_positions.len(), vertex_normals.len());
        assert_eq!(triangle_indices.len(), triangle_collider.len());
        assert_eq!(triangle_indices.len(), triangle_normals.len());
        assert_eq!(triangle_indices.len(), triangle_opposites.len());
        assert_eq!(triangle_indices.len(), triangle_frictions.len());
        assert!(triangle_indices.iter().all(|indices| {
            indices
                .iter()
                .all(|&index| (index as usize) < vertex_positions.len())
        }));
        assert!(triangle_opposites.iter().all(|indices| {
            indices
                .iter()
                .filter(|&&i| i != u32::MAX)
                .all(|&index| (index as usize) < triangle_indices.len())
        }));

        let vertices_3d: Vec<_> = vertex_positions.iter().map(Vector4::xyz).collect();
        let aabbs =
            triangles_to_leaf_aabbs(leaf_size, *forget_distance, &vertices_3d, triangle_indices);

        let bvh = BoundingVolumeHierarchy::new(aabbs, leaf_threshold).unwrap();

        let particle_positions = Allocation::new(device, "particle_positions", particle_positions);
        let particle_collider_bits =
            Allocation::new(device, "particle_collider_bits", particle_collider_bits);
        let particle_velocities =
            Allocation::new(device, "particle_velocities", particle_velocities);

        let vertex_positions = Allocation::new(device, "vertex_positions", vertex_positions);
        let vertex_normals = Allocation::new(device, "vertex_normals", vertex_normals);

        let triangle_indices = Allocation::new(device, "triangle_indices", triangle_indices);
        let triangle_collider = Allocation::new(device, "triangle_collider", triangle_collider);
        let triangle_normals = Allocation::new(device, "triangle_normals", triangle_normals);
        let triangle_opposites = Allocation::new(device, "triangle_opposites", triangle_opposites);
        let triangle_frictions = Allocation::new(device, "triangle_frictions", triangle_frictions);

        let bvh = BoundingVolumeHierarchyAllocations::new(device, leaf_size, &bvh);

        Self {
            particle_positions,
            particle_collider_bits,
            particle_velocities,
            vertex_positions,
            vertex_normals,
            triangle_indices,
            triangle_collider,
            triangle_normals,
            triangle_opposites,
            triangle_frictions,
            bvh,
        }
    }
}

pub struct Output;

impl PipelinePart for Collide {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            forget_distance,
            accept_distance,
            time_step,
        }: Settings,
    ) -> Self {
        let_compiled_module!(
            collide,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), //particle_positions
                    (u32::MIN_BINDING_SIZE, false),            //particle_collider_bits
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), //particle_velocities
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), //vertex_positions
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), //vertex_normals
                    (Triangle::MIN_BINDING_SIZE, false),       //triangle_indices
                    (u32::MIN_BINDING_SIZE, false),            //triangle_collider
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), //triangle_normals
                    (Opposites::MIN_BINDING_SIZE, false),      //triangle_opposites
                    (f32::MIN_BINDING_SIZE, false),            //triangle_frictions
                    (BoundingVolumeHierarchyMeta::MIN_BINDING_SIZE, false), // bvh_meta
                    (u32::MIN_BINDING_SIZE, false),            //bvh_nodes
                    (u32::MIN_BINDING_SIZE, false),            //bvh_indices
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("FORGET_DISTANCE", forget_distance as f64),
                    ("ACCEPT_DISTANCE", accept_distance as f64),
                    ("TIME_STEP", time_step as f64),
                ]
            }
        );

        Self {
            collide,
            workgroup_size,
            dispatch_limit,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            particle_positions,
            particle_collider_bits,
            particle_velocities,
            vertex_positions,
            vertex_normals,
            triangle_indices,
            triangle_collider,
            triangle_normals,
            triangle_opposites,
            triangle_frictions,
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
                    particle_positions.binding(),
                    particle_collider_bits.binding(),
                    particle_velocities.binding(),
                    vertex_positions.binding(),
                    vertex_normals.binding(),
                    triangle_indices.binding(),
                    triangle_collider.binding(),
                    triangle_normals.binding(),
                    triangle_opposites.binding(),
                    triangle_frictions.binding(),
                    bvh.meta.binding(),
                    bvh.nodes.binding(),
                    bvh.indices.binding(),
                ],
            ),
            &[],
        );
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: particle_positions.len::<Vector4<f32>>().get() as u32,
        });
        compute_pass.dispatch_workgroups(indirect.x, indirect.y, indirect.z);

        Ok(Output)
    }
}
