// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use rayon::iter::{IndexedParallelIterator as _, IntoParallelRefIterator, ParallelIterator as _};
use squishy_volumes_mesh_util::Triangle;
use squishy_volumes_util::{NORMALIZATION_EPS, profile};
use squishy_volumes_xpu::FrameInput;

use super::*;

impl CpuState {
    pub fn interpolate_input(&mut self, frame_input: &FrameInput) -> Result<(), Error> {
        profile!("interpolate_input");

        let triangle_indices = frame_input.topology().triangle_indices();

        // linear interpolation between a and b
        let factor_b = frame_input.frame_factor(self.time)?;
        let factor_a = 1. - factor_b;

        let particle_goal_positions: Vec<Vector3<f32>> = frame_input
            .goal_positions_start()
            .par_iter()
            .zip(frame_input.goal_positions_end())
            .map(|(a, b)| factor_a * a + factor_b * b)
            .collect();

        let vertex_positions: Vec<Vector3<f32>> = frame_input
            .collider_start()
            .vertex_positions
            .par_iter()
            .zip(&frame_input.collider_end().vertex_positions)
            .map(|(a, b)| factor_a * a + factor_b * b)
            .collect();

        let triangle_normals: Vec<Vector3<f32>> = triangle_indices
            .par_iter()
            .map(|Triangle { a, b, c }| {
                let a = &vertex_positions[*a as usize];
                let b = &vertex_positions[*b as usize];
                let c = &vertex_positions[*c as usize];
                (b - a)
                    .cross(&(c - a))
                    .try_normalize(NORMALIZATION_EPS)
                    .unwrap_or(Vector3::zeros())
            })
            .collect();

        // Important to weigh the normals by angle
        // https://github.com/Algebraic-UG/squishy_volumes/issues/313
        let vertex_normals: Vec<Vector3<f32>> = frame_input
            .topology()
            .vertex_triangle_lists()
            .par_iter()
            .enumerate()
            .map(|(vertex_index, triangles)| {
                triangles
                    .iter()
                    .map(|triangle_index| {
                        let triangle = triangle_indices[*triangle_index as usize];
                        let mut others = triangle.iter().filter(|&&i| i != vertex_index as u32);
                        let p = vertex_positions[vertex_index];
                        let a = vertex_positions[*others.next().unwrap() as usize];
                        let b = vertex_positions[*others.next().unwrap() as usize];
                        let angle = (a - p).angle(&(b - p));
                        angle * triangle_normals[*triangle_index as usize]
                    })
                    .sum::<Vector3<f32>>()
                    .try_normalize(NORMALIZATION_EPS)
                    .unwrap_or(Vector3::zeros())
            })
            .collect();

        self.interpolated_input = Some(InterpolatedInput {
            particle_goal_positions,
            vertex_positions,
            vertex_normals,
            triangle_normals,
        });

        Ok(())
    }
}
