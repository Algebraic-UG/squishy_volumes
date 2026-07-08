// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use squishy_volumes_file_frame::ParticleFlags;
use squishy_volumes_mesh_util::{
    DistanceResult, Triangle, distance_to_triangle, segment_distance_result,
};
use squishy_volumes_util::{NORMALIZATION_EPS, collider_bits, profile};
use squishy_volumes_xpu::FrameInput;

use super::*;

impl CpuState {
    pub fn collide(&mut self, time_step: f32, frame_input: &FrameInput) -> Result<(), Error> {
        profile!("collect_insides");

        let topology = frame_input.topology();
        let triangle_indices = topology.triangle_indices();
        let triangle_opposites = topology.triangle_opposites();
        let triangle_collider = topology.triangle_collider();

        let InterpolatedInput {
            vertex_positions,
            vertex_normals,
            triangle_frictions,
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
                let leaf = p.map(|c| (c / frame_input.consts().leaf_size).floor() as i32);
                let triangles_to_check = frame_input.bvh().query(&leaf);
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

                    if distance >= frame_input.consts().forget_distance() {
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
                    let a_n = &vertex_normals[triangle.a as usize];
                    let b_n = &vertex_normals[triangle.b as usize];
                    let c_n = &vertex_normals[triangle.c as usize];
                    let ab_n = if opps.ab != u32::MAX {
                        n + triangle_normals[opps.ab as usize]
                    } else {
                        Vector3::zeros()
                    };
                    let bc_n = if opps.bc != u32::MAX {
                        n + triangle_normals[opps.bc as usize]
                    } else {
                        Vector3::zeros()
                    };
                    let ca_n = if opps.ca != u32::MAX {
                        n + triangle_normals[opps.ca as usize]
                    } else {
                        Vector3::zeros()
                    };

                    let ab = a - b;
                    let bc = b - c;
                    let ca = c - a;

                    let sa = n.dot(&bc.cross(&(c - p))) > 0.;
                    let sb = n.dot(&ca.cross(&(a - p))) > 0.;
                    let sc = n.dot(&ab.cross(&(b - p))) > 0.;

                    let DistanceResult {
                        distance,
                        to_p,
                        normal,
                    } = if sa && sb && sc {
                        DistanceResult {
                            distance: (p - a).dot(n).abs(),
                            to_p: n * (p - a).dot(n),
                            normal: *n,
                        }
                    } else {
                        [
                            segment_distance_result(p, a, b, a_n, &ab_n, b_n),
                            segment_distance_result(p, b, c, b_n, &bc_n, c_n),
                            segment_distance_result(p, c, a, c_n, &ca_n, a_n),
                        ]
                        .into_iter()
                        .min_by(|a, b| a.distance.total_cmp(&b.distance))
                        .unwrap()
                    };

                    if normal == Vector3::zeros() {
                        collider_bits::set(collider_bits, collider, None);
                        continue;
                    }

                    let new_side = 0. <= to_p.dot(&normal);
                    let Some(prior_side) = collider_bits::get(*collider_bits, collider) else {
                        if distance < frame_input.consts().accept_distance() {
                            collider_bits::set(collider_bits, collider, Some(new_side));
                        }
                        continue;
                    };

                    if prior_side == new_side {
                        continue;
                    }

                    if distance > NORMALIZATION_EPS {
                        let to_p_normalized = to_p / distance;
                        let tangential =
                            *velocity - to_p_normalized * velocity.dot(&to_p_normalized);
                        *velocity -=
                            tangential * (distance * triangle_frictions[closest_triangle]).min(1.);
                    }

                    *velocity -= to_p / time_step;
                }
            });

        Ok(())
    }
}
