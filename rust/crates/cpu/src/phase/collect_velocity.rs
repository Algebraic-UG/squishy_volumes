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
    // Update the particles' velocity and velocity gradients to be transported.
    pub fn collect_velocity(&mut self) {
        profile!("collect_velocity");
        let grid_node_size = self.frame_input.header().consts.scaled_grid_node_size();
        self.particles
            .positions
            .par_iter()
            .zip(&self.particles.collider_bits)
            .zip(&mut self.particles.velocities)
            .zip(&mut self.particles.velocity_gradients)
            .zip(&self.particles.flags)
            .filter_map(|(e, flags)| (!flags.contains(ParticleFlags::TOMBSTONED)).then_some(e))
            .for_each(
                |(((position, &collider_bits), velocity), velocity_gradient)| {
                    let nodes = quadratic_stencil(position, grid_node_size).map(|node_id| {
                        let (weight, to_grid_node) =
                            node_weight(&node_id, position, grid_node_size);
                        let grid_key = GridKey {
                            node_id,
                            collider_bits,
                        };
                        let grid_index = self.grid_nodes.map.get(&grid_key).expect("missing node");
                        let grid_velocity = self.grid_nodes.velocities[*grid_index as usize];
                        (weight, to_grid_node, grid_velocity)
                    });
                    (*velocity, *velocity_gradient) = gather_from_nodes(nodes, grid_node_size);
                },
            );
    }
}
