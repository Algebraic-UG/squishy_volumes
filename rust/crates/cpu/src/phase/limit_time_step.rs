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
use squishy_volumes_util::{ParticleFlags, profile};

use super::*;

impl CpuState {
    pub fn limit_time_step_before_force(&mut self) {
        profile!("limit_time_step_before_force");

        let grid_node_size = self.frame_input.header().consts.scaled_grid_node_size();

        self.adaptive_time_step_state.time_step_by_sound =
            self.limit_time_step_by_speed_of_sound(grid_node_size);
        self.adaptive_time_step_state.time_step_by_isolated =
            self.limit_time_step_by_isolated_particles(grid_node_size);

        self.adaptive_time_step_state.push_current_limit();
    }

    fn limit_time_step_by_speed_of_sound(&self, grid_node_size: f32) -> Option<f32> {
        profile!("limit_time_step_by_speed_of_sound");
        self.particles
            .parameters
            .par_iter()
            .zip(&self.particles.position_gradients)
            .zip(&self.particles.flags)
            .filter(|(_, flags)| !flags.contains(ParticleFlags::TOMBSTONED))
            .map(|((parameters, position_gradient), flags)| {
                squishy_volumes_util::limit_time_step_by_speed_of_sound(
                    flags,
                    parameters,
                    position_gradient,
                    grid_node_size,
                )
            })
            .min_by(f32::total_cmp)
    }

    fn limit_time_step_by_isolated_particles(&self, grid_node_size: f32) -> Option<f32> {
        profile!("limit_time_step_by_isolated_particles");
        self.particles
            .parameters
            .par_iter()
            .zip(&self.particles.position_gradients)
            .zip(&self.particles.flags)
            .filter(|(_, flags)| !flags.contains(ParticleFlags::TOMBSTONED))
            .map(|((parameters, position_gradient), flags)| {
                squishy_volumes_util::limit_time_step_by_isolated_particles(
                    flags,
                    parameters,
                    position_gradient,
                    grid_node_size,
                )
            })
            .min_by(f32::total_cmp)
    }

    pub fn limit_time_step_before_integrate(&mut self) {
        profile!("limit_time_step_before_integrate");
        let grid_node_size = self.frame_input.header().consts.scaled_grid_node_size();
        self.adaptive_time_step_state.time_step_by_velocity =
            self.limit_time_step_by_velocity(grid_node_size);
        self.adaptive_time_step_state.time_step_by_deformation =
            self.limit_time_step_by_deformation();

        self.adaptive_time_step_state.push_current_limit();
    }

    fn limit_time_step_by_velocity(&self, grid_node_size: f32) -> Option<f32> {
        self.particles
            .velocities
            .par_iter()
            .zip(&self.particles.flags)
            .filter_map(|(e, flags)| (!flags.contains(ParticleFlags::TOMBSTONED)).then_some(e))
            .map(|velocity| {
                squishy_volumes_util::limit_time_step_by_velocity(velocity, grid_node_size)
            })
            .min_by(f32::total_cmp)
    }

    fn limit_time_step_by_deformation(&self) -> Option<f32> {
        self.particles
            .velocity_gradients
            .par_iter()
            .zip(&self.particles.flags)
            .filter_map(|(e, flags)| (!flags.contains(ParticleFlags::TOMBSTONED)).then_some(e))
            .map(|velocity_gradient| {
                squishy_volumes_util::limit_time_step_by_deformation(velocity_gradient)
            })
            .min_by(f32::total_cmp)
    }
}
