// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Result, ensure};
use nalgebra::Vector3;

use crate::{
    input_interpolation::{InterpolatedInput, InterpolatedInputParticles},
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
            .map(|point| &point.input_frame)
            .expect("there's always the a point");

        let gravity;
        let particles_input;
        if let Some(b) = phase_input
            .input_interpolation
            .b()
            .map(|point| &point.input_frame)
        {
            // linear interpolation between a and b
            let factor_b = (frame_time % 1.) as f32;
            let factor_a = 1. - factor_b;

            gravity = factor_a * a.gravity + factor_b * b.gravity;

            particles_input = a
                .particles_input
                .iter()
                .zip(b.particles_input.iter())
                .map(|((name_a, input_a), (name_b, input_b))| {
                    ensure!(name_a == name_b);
                    ensure!(input_a.transforms.len() == input_b.transforms.len());
                    ensure!(input_a.goal_stiffnesses.len() == input_b.goal_stiffnesses.len());

                    let goal_positions = input_a
                        .goal_positions
                        .chunks_exact(3)
                        .map(Vector3::from_column_slice)
                        .zip(
                            input_b
                                .goal_positions
                                .chunks_exact(3)
                                .map(Vector3::from_column_slice),
                        )
                        .map(|(position_a, position_b)| {
                            factor_a * position_a + factor_b * position_b
                        })
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
        } else {
            // in this case assume a constant extrapolation from a
            gravity = a.gravity;

            particles_input = a
                .particles_input
                .iter()
                .map(|(name, input)| {
                    let goal_positions = input
                        .goal_positions
                        .chunks_exact(3)
                        .map(Vector3::from_column_slice)
                        .collect();

                    let goal_stiffnesses = input.goal_stiffnesses.clone();
                    (
                        name.clone(),
                        InterpolatedInputParticles {
                            goal_positions,
                            goal_stiffnesses,
                        },
                    )
                })
                .collect();
        }

        self.interpolated_input = Some(InterpolatedInput {
            gravity,
            particles_input,
        });

        Ok(self)
    }
}
