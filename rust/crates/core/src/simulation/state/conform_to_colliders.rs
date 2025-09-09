// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use blended_mpm_api::T;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

use crate::simulation::grids::Boundary;

use super::{PhaseInput, State, profile};

impl State {
    // Conform the collider's grids to their scripted velocity,
    // taking stickiness and friction into account.
    pub(super) fn conform_to_colliders(mut self, phase_input: PhaseInput) -> Result<Self> {
        profile!("conform_to_colliders");
        let grid_node_size = phase_input.setup.settings.grid_node_size;

        // TODO: this is just s.t. the vector has some elements
        // (and it's not needed for explicit integration)
        self.grid_momentum
            .boundaries
            .resize(self.grid_momentum.map.len(), Default::default());

        for (collider_idx, (collider, grid_momentum)) in self
            .collider_objects
            .iter()
            .zip(self.grid_collider_momentums.iter_mut())
            .enumerate()
        {
            // TODO: this isn't needed for explicit integration
            grid_momentum
                .boundaries
                .resize(grid_momentum.map.len(), Default::default());
            let keys = grid_momentum.map.keys().collect::<Vec<_>>();
            keys.into_par_iter()
                .zip(&mut grid_momentum.velocities)
                .zip(&mut grid_momentum.boundaries)
                .for_each(|((grid_idx, velocity), boundary)| {
                    let collider_distances = self
                        .grid_collider_distances
                        .get(grid_idx)
                        .expect("missing distance node");
                    let distance_node = collider_distances.try_lock().unwrap();

                    let Some(weighted_distance) =
                        distance_node.weighted_distances.get(&collider_idx)
                    else {
                        *boundary = None;
                        return;
                    };

                    let position = grid_idx.map(|i| i as T) * grid_node_size;

                    let negative_normal =
                        weighted_distance.normal * -weighted_distance.distance.signum();

                    *velocity = collider.conform_velocity(position, *velocity, negative_normal, 0.);

                    // TODO: this isn't needed for explicit integration
                    let point_velocity = collider.kinematic.point_velocity_from_world(position);
                    let collider_value = negative_normal.dot(&point_velocity);
                    *boundary = Some(Boundary {
                        normal: negative_normal,
                        collider_value,
                        condition_value: velocity.dot(&negative_normal) - collider_value,
                        dual_variable: 1.,
                    });
                });
        }

        self.grid_collider_distances
            .iter_mut()
            .for_each(|(grid_idx, distances)| {
                let Some(momentum_idx) = self.grid_momentum.map.get(grid_idx) else {
                    return;
                };
                let position = grid_idx.map(|i| i as T) * grid_node_size;

                let Some((collider_idx, weighted_distance)) = distances
                    .get_mut()
                    .unwrap()
                    .weighted_distances
                    .iter()
                    .min_by(|a, b| a.1.distance.abs().total_cmp(&b.1.distance.abs()))
                else {
                    return;
                };
                let normal = weighted_distance.normal * weighted_distance.distance.signum();

                let velocity = &mut self.grid_momentum.velocities[*momentum_idx];
                *velocity = self.collider_objects[*collider_idx]
                    .conform_velocity(position, *velocity, normal, 1.);
            });

        Ok(self)
    }
}
