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
use squishy_volumes_util::{
    BoundingVolumeHierarchy,
    bounding_volume_hierarchy::triangles_to_leaf_aabbs,
    mesh::compute_triangle_lists,
    triangle::{Opposites, Triangle},
};

use super::*;

pub struct Step {
    animate_mesh: AnimateMesh,
    collide: Collide,
    prepare_grid: PrepareGrid,
    register_contributors: RegisterContributors,
    prepare_tmp: PrepareTmp,
    scatter: Scatter,
    meld_grid: MeldGrid,
    collect: Collect,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub grid_node_size: f32,
    pub forget_distance: f32,
    pub accept_distance: f32,
    pub time_step: f32,
}

pub struct Parameters {
    pub factor: f32,
}

#[derive(Clone)]
pub struct ColliderInput {
    pub vertex_positions_start: Allocation,
    pub vertex_positions_end: Allocation,
    pub vertex_triangle_offsets: Allocation,
    pub vertex_triangle_lists: Allocation,

    pub triangle_indices: Allocation,
    pub triangle_collider: Allocation,
    pub triangle_opposites: Allocation,
    pub triangle_frictions: Allocation,

    pub bvh: BoundingVolumeHierarchyAllocations,
}

#[derive(Clone)]
pub struct Input {
    pub indirect_particles: Allocation,
    pub particle_masses: Allocation,
    pub particle_initial_volumes: Allocation,
    pub particle_parameters: Allocation,
    pub particle_positions_and_collider_bits: Allocation,
    pub particle_position_gradients: Allocation,
    pub particle_velocities: Allocation,
    pub particle_velocity_gradients: Allocation,

    pub collider_input: Option<ColliderInput>,
}

#[derive(Clone)]
pub struct InputData<'a> {
    pub masses: &'a [f32],
    pub initial_volumes: &'a [f32],
    pub parameters: &'a [particle_parameters::Device],
    pub positions_and_collider_bits: &'a [PositionAndColliderBits],
    pub position_gradients: &'a [Matrix4x3<f32>],
    pub velocities: &'a [Vector4<f32>],
    pub velocity_gradients: &'a [Matrix4x3<f32>],
    pub vertex_positions_start: &'a [Vector4<f32>],
    pub vertex_positions_end: &'a [Vector4<f32>],
    pub triangle_indices: &'a [Triangle],
    pub triangle_collider: &'a [u32],
    pub triangle_opposites: &'a [Opposites],
    pub triangle_frictions: &'a [f32],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        leaf_size: f32,
        leaf_threshold: u32,
        Settings {
            workgroup_size,
            dispatch_limit,
            forget_distance,
            ..
        }: Settings,
        InputData {
            masses,
            initial_volumes,
            parameters,
            positions_and_collider_bits,
            position_gradients,
            velocities,
            velocity_gradients,
            vertex_positions_start,
            vertex_positions_end,
            triangle_indices,
            triangle_collider,
            triangle_opposites,
            triangle_frictions,
        }: InputData,
    ) -> Result<Self, GpuError> {
        check_length!(masses, initial_volumes)?;
        check_length!(masses, parameters)?;
        check_length!(masses, positions_and_collider_bits)?;
        check_length!(masses, position_gradients)?;
        check_length!(masses, velocities)?;
        check_length!(masses, velocity_gradients)?;
        check_length!(vertex_positions_start, vertex_positions_end)?;
        check_length!(triangle_indices, triangle_collider)?;
        check_length!(triangle_indices, triangle_opposites)?;
        check_length!(triangle_indices, triangle_frictions)?;

        {
            let triangle_indices = triangle_indices.iter().flat_map(Triangle::iter);
            check_indices_valid!(triangle_indices, vertex_positions_start)?;
        }
        {
            let triangle_opposites = triangle_opposites
                .iter()
                .flat_map(Opposites::iter)
                .filter(|&&index| index != u32::MAX);
            check_indices_valid!(triangle_opposites, triangle_indices)?;
        }

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

        let make_aabbs = |positions: &[Vector4<f32>]| {
            triangles_to_leaf_aabbs(
                leaf_size,
                forget_distance,
                &positions.iter().map(Vector4::xyz).collect::<Vec<_>>(),
                triangle_indices,
            )
        };
        let aabbs = make_aabbs(vertex_positions_start)
            .into_iter()
            .zip(make_aabbs(vertex_positions_end))
            .map(|(start, end)| start.extend(&end.min).extend(&end.max))
            .collect();

        let bvh = BoundingVolumeHierarchy::new(aabbs, leaf_threshold);

        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: masses.len() as u32,
        });

        let indirect_particles = Allocation::new(device, "indirect_particles", &[indirect])?;
        let particle_masses = Allocation::new(device, "particle_masses", masses)?;
        let particle_initial_volumes =
            Allocation::new(device, "particle_initial_volumes", initial_volumes)?;
        let particle_parameters = Allocation::new(device, "particle_parameters", parameters)?;
        let particle_positions_and_collider_bits = Allocation::new(
            device,
            "particle_positions_and_collider_bits",
            positions_and_collider_bits,
        )?;
        let particle_position_gradients =
            Allocation::new(device, "particle_position_gradients", position_gradients)?;
        let particle_velocities = Allocation::new(device, "particle_velocities", velocities)?;
        let particle_velocity_gradients =
            Allocation::new(device, "particle_velocity_gradients", velocity_gradients)?;

        let vertex_positions_start =
            Allocation::new(device, "vertex_positions_start", vertex_positions_start)?;
        let vertex_positions_end =
            Allocation::new(device, "vertex_positions_end", vertex_positions_end)?;
        let vertex_triangle_offsets =
            Allocation::new(device, "vertex_triangle_offsets", &vertex_triangle_offsets)?;
        let vertex_triangle_lists =
            Allocation::new(device, "vertex_triangle_lists", &vertex_triangle_lists)?;

        let triangle_indices = Allocation::new(device, "triangle_indices", triangle_indices)?;
        let triangle_collider = Allocation::new(device, "triangle_collider", triangle_collider)?;
        let triangle_opposites = Allocation::new(device, "triangle_opposites", triangle_opposites)?;
        let triangle_frictions = Allocation::new(device, "triangle_frictions", triangle_frictions)?;

        let bvh = BoundingVolumeHierarchyAllocations::new(device, leaf_size, &bvh)?;

        Ok(Self {
            indirect_particles,

            particle_masses,
            particle_initial_volumes,
            particle_parameters,
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,

            collider_input: Some(ColliderInput {
                vertex_positions_start,
                vertex_positions_end,
                vertex_triangle_offsets,
                vertex_triangle_lists,
                triangle_indices,
                triangle_collider,
                triangle_opposites,
                triangle_frictions,
                bvh,
            }),
        })
    }
}

pub struct Output {
    pub indirect_nodes: Allocation,
    pub node_ids_and_collider_bits: Allocation,
    pub node_momentums: Allocation,
}

impl PipelinePart for Step {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            grid_node_size,
            forget_distance,
            accept_distance,
            time_step,
        }: Settings,
    ) -> Self {
        let animate_mesh = AnimateMesh::new(
            context,
            animate_mesh::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );
        let collide = Collide::new(
            context,
            collide::Settings {
                workgroup_size,
                dispatch_limit,
                forget_distance,
                accept_distance,
                time_step,
            },
        );
        let prepare_grid = PrepareGrid::new(
            context,
            prepare_grid::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size,
            },
        );
        let register_contributors = RegisterContributors::new(
            context,
            register_contributors::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size,
            },
        );
        let prepare_tmp = PrepareTmp::new(
            context,
            prepare_tmp::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size,
                time_step,
            },
        );
        let scatter = Scatter::new(
            context,
            scatter::Settings {
                workgroup_size,
                grid_node_size,
            },
        );
        let meld_grid = MeldGrid::new(context, meld_grid::Settings { workgroup_size });
        let collect = Collect::new(
            context,
            collect::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size,
                time_step,
            },
        );

        Self {
            animate_mesh,
            collide,
            prepare_grid,
            register_contributors,
            prepare_tmp,
            scatter,
            meld_grid,
            collect,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_particles,
            particle_masses,
            particle_initial_volumes,
            particle_parameters,
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
            collider_input,
        }: Input,
        Parameters { factor }: Parameters,
    ) -> Result<Output, GpuError> {
        let meld_needed = collider_input.is_some();
        if let Some(ColliderInput {
            vertex_positions_start,
            vertex_positions_end,
            vertex_triangle_offsets,
            vertex_triangle_lists,
            triangle_indices,
            triangle_collider,
            triangle_opposites,
            triangle_frictions,
            bvh,
        }) = collider_input
        {
            let animate_mesh::Output {
                vertex_positions,
                vertex_normals,
                triangle_normals,
            } = self.animate_mesh.record(
                context,
                encoder,
                animate_mesh::Input {
                    vertex_positions_start,
                    vertex_positions_end,
                    vertex_triangle_offsets,
                    vertex_triangle_lists,
                    triangle_indices: triangle_indices.clone(),
                },
                animate_mesh::Parameters { factor },
            )?;

            let collide::Output = self.collide.record(
                context,
                encoder,
                collide::Input {
                    particle_positions_and_collider_bits: particle_positions_and_collider_bits
                        .clone(),
                    particle_velocities: particle_velocities.clone(),
                    vertex_positions,
                    vertex_normals,
                    triangle_indices,
                    triangle_collider,
                    triangle_normals,
                    triangle_opposites,
                    triangle_frictions,
                    bvh,
                },
                collide::Parameters,
            )?;
        }

        let prepare_grid::Output {
            indirect_nodes,
            hash_table,
            node_ids_and_collider_bits,
            hash_table_multi,
            multi_offsets,
            multi,
        } = self.prepare_grid.record(
            context,
            encoder,
            prepare_grid::Input {
                indirect_particles,
                particle_positions_and_collider_bits: particle_positions_and_collider_bits.clone(),
            },
            prepare_grid::Parameters,
        )?;

        let register_contributors::Output {
            contributor_offsets,
            contributors,
        } = self.register_contributors.record(
            context,
            encoder,
            register_contributors::Input {
                indirect_nodes: indirect_nodes.clone(),
                particle_positions_and_collider_bits: particle_positions_and_collider_bits.clone(),
                hash_table: hash_table.clone(),
                node_ids_and_collider_bits: node_ids_and_collider_bits.clone(),
            },
            register_contributors::Parameters,
        )?;

        let prepare_tmp::Output { particle_tmp } = self.prepare_tmp.record(
            context,
            encoder,
            prepare_tmp::Input {
                particle_masses,
                particle_initial_volumes,
                particle_parameters,
                particle_positions_and_collider_bits: particle_positions_and_collider_bits.clone(),
                particle_position_gradients: particle_position_gradients.clone(),
                particle_velocities: particle_velocities.clone(),
                particle_velocity_gradients: particle_velocity_gradients.clone(),
            },
            prepare_tmp::Parameters,
        )?;

        let scatter::Output { node_momentums } = self.scatter.record(
            context,
            encoder,
            scatter::Input {
                indirect_nodes: indirect_nodes.clone(),
                contributor_offsets,
                contributors,
                node_ids_and_collider_bits: node_ids_and_collider_bits.clone(),
                particle_tmp,
            },
            scatter::Parameters,
        )?;

        let node_momentums = if meld_needed {
            let meld_grid::Output {
                node_momentums_out: node_momentums,
            } = self.meld_grid.record(
                context,
                encoder,
                meld_grid::Input {
                    indirect_nodes: indirect_nodes.clone(),
                    node_ids_and_collider_bits: node_ids_and_collider_bits.clone(),
                    hash_table_multi,
                    multi_offsets,
                    multi,
                    node_momentums_in: node_momentums,
                },
                meld_grid::Parameters,
            )?;
            node_momentums
        } else {
            node_momentums
        };

        let collect::Output = self.collect.record(
            context,
            encoder,
            collect::Input {
                hash_table,
                node_ids_and_collider_bits: node_ids_and_collider_bits.clone(),
                node_momentums: node_momentums.clone(),
                particle_positions_and_collider_bits,
                particle_position_gradients,
                particle_velocities,
                particle_velocity_gradients,
            },
            collect::Parameters,
        )?;

        Ok(Output {
            indirect_nodes,
            node_ids_and_collider_bits,
            node_momentums,
        })
    }
}
