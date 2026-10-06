// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use squishy_volumes_util::{ParticleFlags, profile};

use super::*;

impl CpuState {
    pub fn limit_time_step_before_force(&mut self) {
        profile!("limit_time_step_before_force");

        let grid_node_size = self.frame_input.consts().scaled_grid_node_size();

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
        let grid_node_size = self.frame_input.consts().scaled_grid_node_size();
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
