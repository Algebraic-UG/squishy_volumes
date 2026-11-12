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
use squishy_volumes_util::{ParticleFlags, apply_external_force, profile};

use super::*;

impl CpuState {
    pub fn external_force(&mut self) -> Result<(), Error> {
        profile!("external_force");
        let time_step = self.adaptive_time_step_state.allowed_time_step();
        let globals = &self.animated_globals;

        self.particles
            .positions
            .par_iter()
            .zip(&mut self.particles.velocities)
            .zip(&self.particles.goal_positions)
            .zip(&self.particles.flags)
            .filter(|(_, flags)| !flags.contains(ParticleFlags::TOMBSTONED))
            .for_each(|(((position, velocity), goal_position), flags)| {
                let to_goal = flags
                    .contains(ParticleFlags::HAS_GOAL)
                    .then(|| goal_position - position);
                *velocity = apply_external_force(velocity, to_goal.as_ref(), globals, time_step);
            });

        Ok(())
    }
}
