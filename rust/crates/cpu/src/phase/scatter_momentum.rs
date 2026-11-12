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

use nalgebra::Vector3;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator as _, ParallelIterator};
use squishy_volumes_util::profile;

use super::*;

impl CpuState {
    // Mass and velocity transported by particles is scattered to the grids.
    // In explicit time integration the forces can be applied at the same time.
    pub fn scatter_momentum(&mut self) {
        profile!("scatter_momentum");
        let grid_node_size = self.frame_input.header().consts.scaled_grid_node_size();
        let time_step = self.adaptive_time_step_state.allowed_time_step();

        self.grid_nodes.masses = vec![0.; self.grid_nodes.map.len()];
        self.grid_nodes.velocities = vec![Vector3::zeros(); self.grid_nodes.map.len()];
        self.grid_nodes
            .keys
            .par_iter()
            .zip(&mut self.grid_nodes.contributors)
            .zip(&mut self.grid_nodes.masses)
            .zip(&mut self.grid_nodes.velocities)
            .for_each(
                |(((GridKey { node_id, .. }, contributors), mass), velocity)| {
                    for &particle_idx in contributors.get_mut().unwrap().iter() {
                        let particle_idx = particle_idx as usize;
                        let (weight, to_grid_node) = node_weight(
                            node_id,
                            &self.particles.positions[particle_idx],
                            grid_node_size,
                        );

                        let parameters = &self.particles.parameters[particle_idx];
                        let velocity_gradient = &self.particles.velocity_gradients[particle_idx];
                        let stress = kirchhoff_stress(
                            self.particles.flags[particle_idx],
                            parameters,
                            &self.particles.position_gradients[particle_idx],
                            velocity_gradient,
                        );

                        let stress_impulse = StressImpulse {
                            kirchhoff_stress: &stress,
                            initial_volume: parameters.initial_volume(),
                            time_step,
                            grid_node_size,
                        };
                        let momentum = affine_momentum(
                            &to_grid_node,
                            parameters.mass(),
                            &self.particles.velocities[particle_idx],
                            velocity_gradient,
                        ) - stress_impulse.towards(&to_grid_node);

                        *mass += weight * parameters.mass();
                        *velocity += momentum * weight;
                    }
                },
            );
    }
}
