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
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use squishy_volumes_util::{collider_bits, profile};

use super::*;

impl CpuState {
    pub fn meld_grid(&mut self) {
        profile!("meld_grid_nodes");
        let masses = self.grid_nodes.masses.clone();
        let velocities = self.grid_nodes.velocities.clone();

        self.grid_nodes
            .keys
            .par_iter()
            .zip(&mut self.grid_nodes.masses)
            .zip(&mut self.grid_nodes.velocities)
            .enumerate()
            .for_each(
                |(
                    index,
                    (
                        (
                            GridKey {
                                node_id,
                                collider_bits,
                            },
                            mass,
                        ),
                        velocity,
                    ),
                )| {
                    for &other in self
                        .grid_nodes
                        .multi_map
                        .get(node_id)
                        .expect("missing node in multi_map")
                    {
                        let other = other as usize;
                        if other == index {
                            continue;
                        }
                        if !collider_bits::compatible(
                            collider_bits,
                            &self.grid_nodes.keys[other].collider_bits,
                        ) {
                            continue;
                        }
                        *mass += masses[other];
                        *velocity += velocities[other];
                    }

                    if *mass > 0. {
                        *velocity /= *mass;
                    } else {
                        // Numerical edge case
                        *velocity = Vector3::zeros();
                    }
                },
            );
    }
}
