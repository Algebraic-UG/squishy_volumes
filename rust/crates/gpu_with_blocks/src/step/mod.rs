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
    sort_positions_into_cells: SortPositionsIntoCells,
    permute_particles: PermuteParticles,
    animate_mesh: AnimateMesh,
    collide: Collide,
    prepare_grid: PrepareGrid,
    scatter: Scatter,
    collect: Collect,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub bit_count: NonZeroU32,
    pub cell_size: f32,
    pub forget_distance: f32,
    pub accept_distance: f32,
    pub time_step: f32,
}

pub struct Parameters {
    pub factor: f32,
}

pub struct Input {
    pub indirect_particles: Allocation,
    pub indices_in: Allocation,
    pub collider_bits_in: Allocation,
    pub masses_in: Allocation,
    pub initial_volumes_in: Allocation,
    pub parameters_in: Allocation,
    pub positions_in: Allocation,
    pub position_gradients_in: Allocation,
    pub velocities_in: Allocation,
    pub velocity_gradients_in: Allocation,
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
pub struct InputData<'a> {
    pub indices: &'a [u32],
    pub collider_bits: &'a [u32],
    pub masses: &'a [f32],
    pub initial_volumes: &'a [f32],
    pub parameters: &'a [particle_parameters::Device],
    pub positions: &'a [Vector4<f32>],
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
            indices,
            collider_bits,
            masses,
            initial_volumes,
            parameters,
            positions,
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
    ) -> Self {
        assert_eq!(indices.len(), masses.len());
        assert_eq!(indices.len(), collider_bits.len());
        assert_eq!(indices.len(), initial_volumes.len());
        assert_eq!(indices.len(), parameters.len());
        assert_eq!(indices.len(), positions.len());
        assert_eq!(indices.len(), position_gradients.len());
        assert_eq!(indices.len(), velocities.len());
        assert_eq!(indices.len(), velocity_gradients.len());
        assert_eq!(vertex_positions_start.len(), vertex_positions_end.len());
        assert_eq!(triangle_indices.len(), triangle_collider.len());
        assert_eq!(triangle_indices.len(), triangle_opposites.len());
        assert_eq!(triangle_indices.len(), triangle_frictions.len());
        assert!(triangle_indices.iter().all(|indices| {
            indices
                .iter()
                .all(|&index| (index as usize) < vertex_positions_start.len())
        }));
        assert!(triangle_opposites.iter().all(|indices| {
            indices
                .iter()
                .filter(|&&i| i != u32::MAX)
                .all(|&index| (index as usize) < triangle_indices.len())
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

        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: indices.len() as u32,
        });

        let indirect_particles = Allocation::new(device, "indirect_particles", &[indirect]);
        let indices_in = Allocation::new(device, "indices_in", indices);
        let collider_bits_in = Allocation::new(device, "collider_bits_in", collider_bits);
        let masses_in = Allocation::new(device, "masses_in", masses);
        let initial_volumes_in = Allocation::new(device, "initial_volumes_in", initial_volumes);
        let parameters_in = Allocation::new(device, "parameters_in", parameters);
        let positions_in = Allocation::new(device, "positions_in", positions);
        let position_gradients_in =
            Allocation::new(device, "position_gradients_in", position_gradients);
        let velocities_in = Allocation::new(device, "velocities_in", velocities);
        let velocity_gradients_in =
            Allocation::new(device, "velocity_gradients_in", velocity_gradients);

        let vertex_positions_start =
            Allocation::new(device, "vertex_positions_start", vertex_positions_start);
        let vertex_positions_end =
            Allocation::new(device, "vertex_positions_end", vertex_positions_end);
        let vertex_triangle_offsets =
            Allocation::new(device, "vertex_triangle_offsets", &vertex_triangle_offsets);
        let vertex_triangle_lists =
            Allocation::new(device, "vertex_triangle_lists", &vertex_triangle_lists);

        let triangle_indices = Allocation::new(device, "triangle_indices", triangle_indices);
        let triangle_collider = Allocation::new(device, "triangle_collider", triangle_collider);
        let triangle_opposites = Allocation::new(device, "triangle_opposites", triangle_opposites);
        let triangle_frictions = Allocation::new(device, "triangle_frictions", triangle_frictions);

        let bvh = BoundingVolumeHierarchyAllocations::new(device, leaf_size, &bvh);

        Self {
            indirect_particles,
            indices_in,
            collider_bits_in,
            masses_in,
            initial_volumes_in,
            parameters_in,
            positions_in,
            position_gradients_in,
            velocities_in,
            velocity_gradients_in,
            vertex_positions_start,
            vertex_positions_end,
            vertex_triangle_offsets,
            vertex_triangle_lists,
            triangle_indices,
            triangle_collider,
            triangle_opposites,
            triangle_frictions,
            bvh,
        }
    }
}

pub struct Output {
    pub indices_out: Allocation,
    pub collider_bits_out: Allocation,
    pub masses_out: Allocation,
    pub initial_volumes_out: Allocation,
    pub parameters_out: Allocation,
    pub positions_out: Allocation,
    pub position_gradients_out: Allocation,
    pub velocities_out: Allocation,
    pub velocity_gradients_out: Allocation,
}

pub struct OutputData {
    pub indices_out: Vec<u32>,
    pub masses_out: Vec<f32>,
    pub initial_volumes_out: Vec<f32>,
    pub parameters_out: Vec<particle_parameters::Device>,
    pub positions_out: Vec<Vector4<f32>>,
    pub position_gradients_out: Vec<Matrix4x3<f32>>,
    pub velocities_out: Vec<Vector4<f32>>,
    pub velocity_gradients_out: Vec<Matrix4x3<f32>>,
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
            bit_count,
            cell_size,
            forget_distance,
            accept_distance,
            time_step,
        }: Settings,
    ) -> Self {
        let sort_positions_into_cells = SortPositionsIntoCells::new(
            context,
            sort_positions_into_cells::Settings {
                workgroup_size,
                dispatch_limit,
                cell_size,
                bit_count,
            },
        );
        let permute_particles =
            PermuteParticles::new(context, permute_particles::Settings { workgroup_size });
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
                cell_size,
            },
        );
        let scatter = Scatter::new(
            context,
            scatter::Settings {
                workgroup_size,
                cell_size,
                time_step,
            },
        );
        let collect = Collect::new(
            context,
            collect::Settings {
                workgroup_size,
                cell_size,
                time_step,
            },
        );

        Self {
            sort_positions_into_cells,
            permute_particles,
            animate_mesh,
            collide,
            prepare_grid,
            scatter,
            collect,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_particles,
            indices_in,
            collider_bits_in,
            masses_in,
            initial_volumes_in,
            parameters_in,
            positions_in,
            position_gradients_in,
            velocities_in,
            velocity_gradients_in,
            vertex_positions_start,
            vertex_positions_end,
            vertex_triangle_offsets,
            vertex_triangle_lists,
            triangle_indices,
            triangle_collider,
            triangle_opposites,
            triangle_frictions,
            bvh,
        }: Input,
        Parameters { factor }: Parameters,
    ) -> Result<Output, GpuError> {
        let sort_positions_into_cells::Output { permutation } =
            self.sort_positions_into_cells.record(
                context,
                encoder,
                sort_positions_into_cells::Input {
                    indirect: indirect_particles.clone(),
                    positions: positions_in.clone(),
                },
                sort_positions_into_cells::Parameters,
            )?;

        let permute_particles::Output {
            indices_out,
            collider_bits_out,
            masses_out,
            initial_volumes_out,
            parameters_out,
            positions_out,
            position_gradients_out,
            velocities_out,
            velocity_gradients_out,
        } = self.permute_particles.record(
            context,
            encoder,
            permute_particles::Input {
                indirect: indirect_particles.clone(),
                permutation,
                indices_in,
                collider_bits_in,
                masses_in,
                initial_volumes_in,
                parameters_in,
                positions_in,
                position_gradients_in,
                velocities_in,
                velocity_gradients_in,
            },
            permute_particles::Parameters,
        )?;

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
                particle_positions: positions_out.clone(),
                particle_collider_bits: collider_bits_out.clone(),
                particle_velocities: velocities_out.clone(),
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

        let prepare_grid::Output {
            indirect_cells,
            indirect_cells_batch,
            indirect_colors,
            indirect_colors_batch,
            indirect_blocks,
            cell_indices,
            cell_index_ranges,
            cell_ids,
            block_ids,
            block_table,
        } = self.prepare_grid.record(
            context,
            encoder,
            prepare_grid::Input {
                indirect_particles,
                positions: positions_out.clone(),
            },
            prepare_grid::Parameters,
        )?;
        drop(indirect_cells);
        drop(indirect_colors);
        let scatter::Output { blocks } = self.scatter.record(
            context,
            encoder,
            scatter::Input {
                indirect_colors_batch,
                cell_indices,
                cell_index_ranges: cell_index_ranges.clone(),
                cell_ids: cell_ids.clone(),
                block_ids: block_ids.clone(),
                block_table: block_table.clone(),
                masses: masses_out.clone(),
                initial_volumes: initial_volumes_out.clone(),
                particle_parameters: parameters_out.clone(),
                positions: positions_out.clone(),
                position_gradients: position_gradients_out.clone(),
                velocities: velocities_out.clone(),
                velocity_gradients: velocity_gradients_out.clone(),
            },
            scatter::Parameters,
        )?;

        let collect::Output {
            positions: positions_out,
            position_gradients: position_gradients_out,
            velocities: velocities_out,
            velocity_gradients: velocity_gradients_out,
        } = self.collect.record(
            context,
            encoder,
            collect::Input {
                indirect_cells_batch,
                cell_index_ranges,
                cell_ids,
                block_ids,
                block_table,
                positions: positions_out,
                position_gradients: position_gradients_out,
                velocities: velocities_out,
                velocity_gradients: velocity_gradients_out,
                blocks,
            },
            collect::Parameters,
        )?;

        Ok(Output {
            indices_out,
            collider_bits_out,
            masses_out,
            initial_volumes_out,
            parameters_out,
            positions_out,
            position_gradients_out,
            velocities_out,
            velocity_gradients_out,
        })
    }
}
