// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use nalgebra::Vector3;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use squishy_volumes_util::collider_bits;

use crate::{profile, state::grids::GridKey};

use super::{PhaseInput, State};

impl State {
    pub fn meld_grid(mut self, _phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("meld_grid");
        let masses = self.grid.masses.clone();
        let velocities = self.grid.velocities.clone();

        self.grid
            .keys
            .par_iter()
            .zip(&mut self.grid.masses)
            .zip(&mut self.grid.velocities)
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
                        .grid
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
                            &self.grid.keys[other].collider_bits,
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
        Ok(self)
    }
}
