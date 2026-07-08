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

        let a = frame_input.a();
        let b = frame_input.b().unwrap_or(a);

        // linear interpolation between a and b
        let factor_b = frame_input.frame_factor(self.time)?;
        let factor_a = 1. - factor_b;

        let gravity = factor_a * a.gravity() + factor_b * b.gravity();

        let particle_goal_positions: Vec<Vector3<f32>> = a
            .particle_goal_positions()
            .par_iter()
            .zip(b.particle_goal_positions())
            .map(|(a, b)| factor_a * a + factor_b * b)
            .collect();

        let vertex_positions: Vec<Vector3<f32>> = a
            .vertex_positions()
            .par_iter()
            .zip(b.vertex_positions())
            .map(|(a, b)| factor_a * a + factor_b * b)
            .collect();

        let triangle_normals: Vec<Vector3<f32>> = frame_input
            .topology()
            .triangle_indices()
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

        let vertex_normals: Vec<Vector3<f32>> = frame_input
            .topology()
            .vertex_triangle_lists()
            .par_iter()
            .map(|triangles| {
                triangles
                    .iter()
                    .map(|triangle_index| triangle_normals[*triangle_index as usize])
                    .sum::<Vector3<f32>>()
                    .try_normalize(NORMALIZATION_EPS)
                    .unwrap_or(Vector3::zeros())
            })
            .collect();

        let triangle_frictions: Vec<f32> = a
            .triangle_frictions()
            .par_iter()
            .zip(b.triangle_frictions())
            .map(|(a, b)| factor_a * a + factor_b * b)
            .collect();

        self.interpolated_input = Some(InterpolatedInput {
            gravity,
            particle_goal_positions,
            vertex_positions,
            vertex_normals,
            triangle_frictions,
            triangle_normals,
        });

        Ok(())
    }
}
