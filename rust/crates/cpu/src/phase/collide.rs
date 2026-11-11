// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use nalgebra::Vector3;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use squishy_volumes_mesh_util::{
    DistanceResult, FeatureNormals, Triangle, barycentric_coordinates, closest_feature,
    distance_to_triangle, edge_pseudo_normal,
};
use squishy_volumes_util::{
    Contact, NORMALIZATION_EPS, ParticleFlags, collider_bits, collision_response, profile,
};

use super::*;

impl CpuState {
    pub fn collide(&mut self) {
        profile!("collect_insides");

        let time_step = self.adaptive_time_step_state.allowed_time_step();

        let topology = self.frame_input.topology();
        let triangle_indices = topology.triangle_indices();
        let triangle_opposites = topology.triangle_opposites();
        let triangle_collider = topology.triangle_collider();

        let triangle_frictions = &self.frame_input.collider_start().triangle_frictions;
        let triangle_dampings = &self.frame_input.collider_start().triangle_dampings;

        let vertex_velocities = self.frame_input.vertex_velocities();

        let InterpolatedInput {
            vertex_positions,
            vertex_normals,
            triangle_normals,
            ..
        } = self
            .interpolated_input
            .as_ref()
            .expect("no input interpolated");

        self.particles
            .positions
            .par_iter()
            .zip(&mut self.particles.velocities)
            .zip(&mut self.particles.collider_bits)
            .zip(&self.particles.flags)
            .filter_map(|(e, flags)| (!flags.contains(ParticleFlags::TOMBSTONED)).then_some(e))
            .for_each(|((p, velocity), collider_bits)| {
                let leaf =
                    p.map(|c| (c / self.frame_input.header().consts.leaf_size).floor() as i32);
                let triangles_to_check = self.frame_input.bvh().query(&leaf);
                if triangles_to_check.is_empty() {
                    *collider_bits = 0;
                    return;
                }

                let mut closest_triangle_per_collider: [u32; 16] = [u32::MAX; 16];
                let mut min_distance_per_collider: [f32; 16] = [f32::MAX; 16];

                for triangle_index in triangles_to_check {
                    let n = &triangle_normals[*triangle_index as usize];
                    if *n == Vector3::zeros() {
                        continue;
                    }

                    let Triangle { a, b, c } = &triangle_indices[*triangle_index as usize];

                    let distance = distance_to_triangle(
                        p,
                        &vertex_positions[*a as usize],
                        &vertex_positions[*b as usize],
                        &vertex_positions[*c as usize],
                        n,
                    );

                    if distance >= self.frame_input.header().consts.forget_distance() {
                        continue;
                    }

                    let collider = triangle_collider[*triangle_index as usize] as usize;
                    if distance < min_distance_per_collider[collider] {
                        min_distance_per_collider[collider] = distance;
                        closest_triangle_per_collider[collider] = *triangle_index;
                    }
                }

                for (collider, closest_triangle) in
                    closest_triangle_per_collider.into_iter().enumerate()
                {
                    if closest_triangle == u32::MAX {
                        collider_bits::set(collider_bits, collider, None);
                        continue;
                    }
                    let closest_triangle = closest_triangle as usize;

                    let triangle = &triangle_indices[closest_triangle];

                    let opps = &triangle_opposites[closest_triangle];
                    let n = &triangle_normals[closest_triangle];
                    let a = &vertex_positions[triangle.a as usize];
                    let b = &vertex_positions[triangle.b as usize];
                    let c = &vertex_positions[triangle.c as usize];
                    let a_v = &vertex_velocities[triangle.a as usize];
                    let b_v = &vertex_velocities[triangle.b as usize];
                    let c_v = &vertex_velocities[triangle.c as usize];
                    let a_n = &vertex_normals[triangle.a as usize];
                    let b_n = &vertex_normals[triangle.b as usize];
                    let c_n = &vertex_normals[triangle.c as usize];
                    let edge_normal = |opposite: u32| {
                        let neighbour_normal =
                            (opposite != u32::MAX).then(|| &triangle_normals[opposite as usize]);
                        edge_pseudo_normal(n, neighbour_normal)
                    };

                    let barycentric = barycentric_coordinates(p, [a, b, c], n);
                    let DistanceResult {
                        distance,
                        to_p,
                        normal,
                    } = closest_feature(
                        p,
                        [a, b, c],
                        n,
                        &FeatureNormals {
                            vertices: [*a_n, *b_n, *c_n],
                            edges: [opps.ab, opps.bc, opps.ca].map(edge_normal),
                        },
                    );

                    if normal == Vector3::zeros() {
                        collider_bits::set(collider_bits, collider, None);
                        continue;
                    }

                    let new_side = 0. <= to_p.dot(&normal);
                    let Some(prior_side) = collider_bits::get(*collider_bits, collider) else {
                        if distance < self.frame_input.header().consts.accept_distance() {
                            collider_bits::set(collider_bits, collider, Some(new_side));
                        }
                        continue;
                    };

                    if prior_side == new_side {
                        continue;
                    }

                    if distance < NORMALIZATION_EPS {
                        continue;
                    }

                    let contact = Contact {
                        to_p,
                        collider_velocity: a_v * barycentric.x
                            + b_v * barycentric.y
                            + c_v * barycentric.z,
                        friction: triangle_frictions[closest_triangle],
                        damping: triangle_dampings[closest_triangle],
                    };
                    *velocity = collision_response(velocity, &contact, time_step);
                }
            });
    }
}
