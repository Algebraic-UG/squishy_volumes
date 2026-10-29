// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use squishy_volumes_util::{AnimatedGlobals, NORMALIZATION_EPS, ParticleFlags, profile};

use super::*;

impl CpuState {
    pub fn external_force(&mut self) -> Result<(), Error> {
        profile!("external_force");
        let time_step = self.adaptive_time_step_state.allowed_time_step();

        let AnimatedGlobals {
            gravity_x,
            gravity_y,
            gravity_z,
            goal_stiffness,
            goal_damping,
            damping,
        } = self.animated_globals;
        let gravity = nalgebra::Vector3::new(gravity_x, gravity_y, gravity_z);

        self.particles
            .positions
            .par_iter()
            .zip(&mut self.particles.velocities)
            .zip(&self.particles.goal_positions)
            .zip(&self.particles.flags)
            .filter(|(_, flags)| !flags.contains(ParticleFlags::TOMBSTONED))
            .for_each(|(((position, velocity), goal_position), flags)| {
                *velocity -= time_step * damping * *velocity;
                *velocity += time_step * gravity;

                if flags.contains(ParticleFlags::HAS_GOAL) {
                    let to_goal = goal_position - position;
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
