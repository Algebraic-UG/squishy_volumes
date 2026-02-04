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

use crate::{
    kernels::{kernel_quadratic_unrolled, position_to_shift_quadratic},
    profile,
    state::{particles::ParticleState, util::find_worst_incompatibility},
};

use super::{PhaseInput, State};

impl State {
    // To each grid node a number of particles can contribute.
    // We iterate over the particles since it's clear to which nodes a given particle will
    // contribute. Then just use Mutexes to avoid data races.
    pub fn register_contributors(mut self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("register_contributors");
        let grid_node_size = phase_input.consts.grid_node_size;

        // to avoid frequent reallocations we add nodes with generous capacity
        let initial_capacity = 1 << 4;

        {
            profile!("prepare");
            self.grid_momentum.prepare_contributors(initial_capacity);
            self.grid_collider_momentums
                .iter_mut()
                .for_each(|grid| grid.prepare_contributors(initial_capacity));
        }

        self.particles
            .positions
            .par_iter()
            .zip(&self.particles.collider_insides)
            .enumerate()
            .zip(&self.particles.states)
            .filter_map(|(e, state)| (*state != ParticleState::Tombstoned).then_some(e))
            .for_each(|(idx, (position, collider_inside))| {
                let shift = position_to_shift_quadratic(position, grid_node_size);
                kernel_quadratic_unrolled!(|grid_idx| {
                    let grid_idx = grid_idx + shift;
                    let incompatibility =
                        self.grid_collider_distances
                            .get(&grid_idx)
                            .and_then(|grid_node| {
                                find_worst_incompatibility(collider_inside, &grid_node.lock())
                            });

                    let grid = if let Some(collider_idx) = incompatibility {
                        &self.grid_collider_momentums[collider_idx]
                    } else {
                        &self.grid_momentum
                    };

                    let grid_idx = grid.map.get(&grid_idx).expect("missing node");
                    grid.contributors[*grid_idx].lock().push(idx);
                });
            });
        Ok(self)
    }
}
