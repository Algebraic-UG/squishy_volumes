// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::hash_map::Entry;

use anyhow::{Context as _, Result, bail};

use crate::{math::RASTERIZATION_LAYERS, profile, rasterization::rasterize, state::ObjectIndex};

use super::{PhaseInput, State};

impl State {
    // Splat collider distances into the grid.
    // Each grid node then contains signed distance and normal information for each collider.
    // But only if it's close enough.
    pub(super) fn scatter_collider_distances(
        mut self,
        phase_input: &mut PhaseInput,
    ) -> Result<Self> {
        profile!("scatter_collider_distances");
        let grid_node_size = phase_input.consts.grid_node_size;

        self.grid_collider_distances.clear();

        let collider_input = &self
            .interpolated_input
            .as_ref()
            .expect("interpolated input missing")
            .collider_input;

        for (name, input) in collider_input.iter() {
            let object_index = self.name_map.get(name).context("Missing object")?;
            let ObjectIndex::Collider(collider_index) = object_index else {
                bail!("Wrong object type");
            };

            for &[a, b, c] in &input.triangles {
                let corner_a = &input.vertex_positions[a as usize];
                let corner_b = &input.vertex_positions[b as usize];
                let corner_c = &input.vertex_positions[c as usize];

                let normal_a = &input.vertex_normals[a as usize];
                let normal_b = &input.vertex_normals[b as usize];
                let normal_c = &input.vertex_normals[c as usize];

                let order_edge = |[a, b]: [u32; 2]| if a < b { [a, b] } else { [b, a] };
                let pick_other = |a: u32| {
                    move |&[b, c]: &[u32; 2]| {
                        &input.vertex_positions[if b != a { b } else { c } as usize]
                    }
                };
                let opposite_d = input
                    .edges_with_opposites
                    .get(&order_edge([a, b]))
                    .map(pick_other(c));
                let opposite_e = input
                    .edges_with_opposites
                    .get(&order_edge([b, c]))
                    .map(pick_other(a));
                let opposite_f = input
                    .edges_with_opposites
                    .get(&order_edge([c, a]))
                    .map(pick_other(b));

                for (grid_node, weighted_distance) in rasterize(
                    [corner_a, corner_b, corner_c],
                    [normal_a, normal_b, normal_c],
                    [opposite_d, opposite_e, opposite_f],
                    grid_node_size,
                    RASTERIZATION_LAYERS,
                ) {
                    let guard = self
                        .grid_collider_distances
                        .entry(grid_node)
                        .or_default()
                        .get_mut()
                        .unwrap();
                    match guard.weighted_distances.entry(*collider_index) {
                        Entry::Occupied(mut occupied_entry) => {
                            let exsiting = occupied_entry.get_mut();
                            if exsiting.distance.abs() < weighted_distance.distance.abs() {
                                continue;
                            }
                            *exsiting = weighted_distance;
                        }
                        Entry::Vacant(vacant_entry) => {
                            vacant_entry.insert(weighted_distance);
                        }
                    }
                }
            }
        }

        // self.scatter_collider_distances_create_entries(grid_node_size);
        // self.scatter_collider_distances_reset();
        // self.scatter_collider_distances_scatter(grid_node_size);
        Ok(self)
    }

    /*
    fn scatter_collider_distances_create_entries(&mut self, grid_node_size: T) {
        profile!("create_entries");
        for collider in &self.collider_objects {
            if !collider.has_moved {
                continue;
            }
            let new_entries: Vec<Vector3<i32>> = collider
                .surface_samples
                .par_iter()
                .flat_map_iter(|surface_sample| {
                    let shift = position_to_shift_quadratic(
                        &collider
                            .kinematic
                            .to_world_position(surface_sample.position),
                        grid_node_size,
                    );
                    kernel_quadratic_unrolled!(move |grid_idx| grid_idx + shift)
                        .into_iter()
                        .filter(|grid_idx| !self.grid_collider_distances.contains_key(grid_idx))
                })
                .collect();
            self.grid_collider_distances.extend(
                new_entries
                    .into_iter()
                    .map(|grid_idx| (grid_idx, Default::default())),
            );
        }
    }

    fn scatter_collider_distances_reset(&mut self) {
        profile!("reset");
        self.grid_collider_distances
            .values_mut()
            .for_each(|node| node.get_mut().unwrap().weighted_distances.clear());
    }

    // Splat distance information by projecting oriented disks.
    fn scatter_collider_distances_scatter(&self, grid_node_size: T) {
        profile!("scatter");
        for (collider_idx, collider) in self.collider_objects.iter().enumerate() {
            collider
                .surface_samples
                .par_iter()
                .for_each(|SurfaceSample { position, normal }| {
                    let position = collider.kinematic.to_world_position(*position);
                    let normal = collider.kinematic.to_world_normal(*normal);

                    let shift = position_to_shift_quadratic(&position, grid_node_size);

                    kernel_quadratic_unrolled!(move |grid_idx: Vector3<i32>| {
                        let grid_idx = grid_idx + shift;
                        let grid_node_position = grid_idx.map(|i| i as T) * grid_node_size;
                        let to_grid_node = grid_node_position - position;
                        let distance = normal.dot(&to_grid_node);
                        let tangential_part = to_grid_node - normal * distance;
                        if tangential_part.norm() > grid_node_size * SURFACE_DISK_SIZE_FACTOR {
                            // trust that another nearby disk will be a better fit
                            return;
                        }
                        let mut grid_node = self
                            .grid_collider_distances
                            .get(&grid_idx)
                            .expect("missing node")
                            .lock();
                        match grid_node.weighted_distances.entry(collider_idx) {
                            Entry::Occupied(mut occupied_entry) => {
                                if distance.abs() < occupied_entry.get().distance.abs() {
                                    occupied_entry.get_mut().distance = distance;
                                    occupied_entry.get_mut().normal = normal;
                                }
                            }
                            Entry::Vacant(vacant_entry) => {
                                vacant_entry.insert(WeightedDistance { distance, normal });
                            }
                        }
                    });
                });
        }
    }
    */
}
