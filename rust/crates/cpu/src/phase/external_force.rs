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
