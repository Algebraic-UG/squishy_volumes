// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use squishy_volumes_file_frame::ParticleFlags;
use squishy_volumes_util::{AnimatedGlobals, NORMALIZATION_EPS, profile};
use squishy_volumes_xpu::FrameInput;

use super::*;

impl CpuState {
    pub fn external_force(&mut self, frame_input: &FrameInput) -> Result<(), Error> {
        profile!("external_force");
        let time_step = self.adaptive_time_step_state.allowed_time_step();
        let a = frame_input.a();
        let b = frame_input.b().unwrap_or(a);
        let input_flags_a = a.particle_flags();
        let input_flags_b = b.particle_flags();

        let interpolated_input = self
            .interpolated_input
            .as_ref()
            .ok_or(Error::InterpolatedInputMissing)?;
        let AnimatedGlobals {
            gravity_x,
            gravity_y,
            gravity_z,
            goal_stiffness,
            goal_damping,
            damping,
        } = interpolated_input.animated_globals;
        let gravity = nalgebra::Vector3::new(gravity_x, gravity_y, gravity_z);

        self.particles
            .positions
            .par_iter()
            .zip(&mut self.particles.velocities)
            .enumerate()
            .zip(&self.particles.flags)
            .filter_map(|(e, flags)| (!flags.contains(ParticleFlags::TOMBSTONED)).then_some(e))
            .for_each(|(index, (position, velocity))| {
                *velocity -= time_step * damping * *velocity;
                *velocity += time_step * gravity;

                let index = self.particles.sort_map[index] as usize;
                if input_flags_a[index].contains(ParticleFlags::HAS_GOAL)
                    && input_flags_b[index].contains(ParticleFlags::HAS_GOAL)
                {
                    let to_goal = interpolated_input.particle_goal_positions[index] - position;
                    let distance = to_goal.norm();
                    if distance > NORMALIZATION_EPS {
                        let to_goal_dir = to_goal / distance;
                        *velocity += (goal_stiffness * distance
                            - goal_damping * to_goal_dir.dot(velocity))
                            * time_step
                            * to_goal_dir;
                    }
                }
            });

        Ok(())
    }
}
