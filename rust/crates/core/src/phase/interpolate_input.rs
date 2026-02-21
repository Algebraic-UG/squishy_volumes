// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Result, ensure};
use squishy_volumes_api::T;

use crate::{
    input_interpolation::{
        InterpolatedInput, InterpolatedInputCollider, InterpolatedInputParticles,
    },
    profile,
};

use super::{PhaseInput, State};

impl State {
    pub fn interpolate_input(mut self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("interpolate_input");

        let time = self.time;
        let frame_time = time * phase_input.consts.frames_per_second as f64;

        // this should be a no-op for all in-between-frame-steps
        phase_input
            .input_interpolation
            .load(frame_time.floor() as usize)?;

        let a = phase_input
            .input_interpolation
            .a()
            .map(|point| &point.interpolant)
            .expect("there's always the a point");

        let Some(b) = phase_input
            .input_interpolation
            .b()
            .map(|point| &point.interpolant)
        else {
            // in this case assume a constant extrapolation from a
            self.interpolated_input = Some(a.clone());
            return Ok(self);
        };

        // linear interpolation between a and b
        let factor_b = (frame_time % 1.) as T;
        let factor_a = 1. - factor_b;

        let gravity = factor_a * a.gravity + factor_b * b.gravity;

        let particles_input = a
            .particles_input
            .iter()
            .zip(b.particles_input.iter())
            .map(|((name_a, input_a), (name_b, input_b))| {
                ensure!(name_a == name_b);
                ensure!(input_a.goal_stiffnesses.len() == input_b.goal_stiffnesses.len());
                ensure!(input_a.goal_positions.len() == input_b.goal_positions.len());

                let goal_positions = input_a
                    .goal_positions
                    .iter()
                    .zip(&input_b.goal_positions)
                    .map(|(position_a, position_b)| factor_a * position_a + factor_b * position_b)
                    .collect();
                let goal_stiffnesses = input_a
                    .goal_stiffnesses
                    .iter()
                    .zip(&input_b.goal_stiffnesses)
                    .map(|(stiffness_a, stiffness_b)| {
                        factor_a * stiffness_a + factor_b * stiffness_b
                    })
                    .collect();

                Ok((
                    name_a.clone(),
                    InterpolatedInputParticles {
                        goal_positions,
                        goal_stiffnesses,
                    },
                ))
            })
            .collect::<Result<_>>()?;

        let collider_input = a
            .collider_input
            .iter()
            .zip(b.collider_input.iter())
            .map(|((name_a, input_a), (name_b, input_b))| {
                ensure!(name_a == name_b);
                let vertex_positions: Vec<_> = input_a
                    .vertex_positions
                    .iter()
                    .zip(&input_b.vertex_positions)
                    .map(|(position_a, position_b)| factor_a * position_a + factor_b * position_b)
                    .collect();
                let vertex_velocities = input_a
                    .vertex_positions
                    .iter()
                    .zip(&input_b.vertex_positions)
                    .map(|(position_a, position_b)| {
                        (position_b - position_a) * phase_input.consts.frames_per_second as T
                    })
                    .collect();

                let vertex_normals = input_a
                    .vertex_normals
                    .iter()
                    .zip(&input_b.vertex_normals)
                    .map(|(normal_a, normal_b)| {
                        if let (Some(normal_a), Some(normal_b)) = (normal_a, normal_b) {
                            Some(
                                normal_a
                                    .try_slerp(normal_b, factor_b, 0.)
                                    .unwrap_or(if factor_b < 0.5 { *normal_a } else { *normal_b }),
                            )
                        } else {
                            None
                        }
                    })
                    .collect();
                let triangle_frictions = input_a
                    .triangle_frictions
                    .iter()
                    .zip(&input_b.triangle_frictions)
                    .map(|(friction_a, friction_b)| factor_a * friction_a + factor_b * friction_b)
                    .collect();
                let triangle_stickynesses = input_a
                    .triangle_stickynesses
                    .iter()
                    .zip(&input_b.triangle_stickynesses)
                    .map(|(stickyness_a, stickyness_b)| {
                        factor_a * stickyness_a + factor_b * stickyness_b
                    })
                    .collect();

                Ok((
                    name_a.clone(),
                    InterpolatedInputCollider {
                        vertex_positions,
                        vertex_normals,
                        vertex_velocities,
                        triangle_frictions,
                        triangle_stickynesses,

                        // assume topology constant from a
                        triangles: input_a.triangles.clone(),
                        edges_with_opposites: input_a.edges_with_opposites.clone(),
                    },
                ))
            })
            .collect::<Result<_>>()?;

        self.interpolated_input = Some(InterpolatedInput {
            gravity,
            particles_input,
            collider_input,
        });

        Ok(self)
    }
}
