// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

use crate::{phase::PhaseInput, profile, state::State};

impl State {
    // Conform the collider's grids to their scripted velocity,
    // taking stickiness and friction into account.
    pub fn conform_to_colliders(mut self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("conform_to_colliders");

        for (collider_idx, grid_momentum) in self.grid_collider_momentums.iter_mut().enumerate() {
            let keys = grid_momentum.map.keys().collect::<Vec<_>>();
            keys.into_par_iter()
                .zip(&mut grid_momentum.velocities)
                .for_each(|(grid_idx, velocity)| {
                    let Some(info) = self
                        .grid_collider
                        .get(grid_idx)
                        .and_then(|node| node.infos.get(&collider_idx))
                    else {
                        return;
                    };

                    let relative_velocity = *velocity - info.velocity;
                    let normal = info.normal * -info.distance.signum();

                    let normal_part = normal.dot(&relative_velocity);
                    let normal_velocity = normal * normal_part;
                    let tangent_velocity = relative_velocity - normal_velocity;

                    // TODO: friction, stickyness
                    if normal_part < 0. {
                        *velocity = (1. + normal_part * phase_input.time_step * info.friction)
                            .max(0.)
                            * tangent_velocity
                    }
                });
        }

        Ok(self)
    }
}
