// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Context, Result, bail};

use crate::{phase::PhaseInput, profile, state::ObjectIndex};

use super::State;

impl State {
    pub fn goal_forces(mut self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("goal_forces");
        let time_step = phase_input.time_step;

        let interpolated_input = self
            .interpolated_input
            .as_ref()
            .context("Missing interpolated input")?;
        for (name, particles_input) in &interpolated_input.particles_input {
            let object_index = self.name_map.get(name).context("Missing object")?;
            let ObjectIndex::Particles(index) = object_index else {
                bail!("Wrong object type");
            };
            for (particle_object_index, particle_world_index) in
                self.particle_objects[*index].particles.iter().enumerate()
            {
                let particle_world_index = self.particles.reverse_sort_map[*particle_world_index];
                let goal_position = particles_input.goal_positions[particle_object_index];
                let goal_stiffness = particles_input.goal_stiffnesses[particle_object_index];
                let position = self.particles.positions[particle_world_index];

                self.particles.velocities[particle_world_index] +=
                    time_step * goal_stiffness * (goal_position - position);
            }
        }
        Ok(self)
    }
}
