// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Context, Result};

use crate::{ParticleFlags, phase::PhaseInput, profile};

use super::State;

impl State {
    pub fn goal_forces(mut self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("goal_forces");
        let time_step = phase_input.time_step;

        let interpolated_input = self
            .interpolated_input
            .as_ref()
            .context("Missing interpolated input")?;
        for (initial_index, (flags, goal_position)) in phase_input
            .input_interpolation
            .a()
            .particle_flags()
            .iter()
            .zip(&interpolated_input.particle_goal_positions)
            .enumerate()
        {
            if !flags.contains(ParticleFlags::HasGoal) {
                continue;
            }

            let current_index = self.particles.reverse_sort_map[initial_index];
            let position = &self.particles.positions[current_index];
            self.particles.velocities[current_index] = (goal_position - position) / time_step;
        }
        Ok(self)
    }
}
