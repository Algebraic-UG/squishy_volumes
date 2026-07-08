// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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
                            &collider_bits,
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
