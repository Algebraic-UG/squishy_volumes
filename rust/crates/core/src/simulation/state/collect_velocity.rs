// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use nalgebra::{Matrix3, Vector3};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use squishy_volumes_api::T;
use std::array::from_fn;

use crate::{simulation::weights::kernel_quadratic, weights::KERNEL_QUADRATIC_LENGTH};

use super::{PhaseInput, State, check_shifted_quadratic, find_worst_incompatibility, profile};

impl State {
    // Update the particles' velocity and velocity gradients to be transported.
    pub(super) fn collect_velocity(mut self, phase_input: PhaseInput) -> Result<Self> {
        profile!("collect_velocity");
        let grid_node_size = phase_input.setup.settings.grid_node_size;
        self.particles
            .positions
            .par_iter()
            .zip(&self.particles.collider_insides)
            .zip(&mut self.particles.velocities)
            .zip(&mut self.particles.velocity_gradients)
            .for_each(
                |(((position, collider_inside), velocity), velocity_gradient)| {
                    *velocity = Vector3::zeros();
                    *velocity_gradient = Matrix3::zeros();

                    let normalized = position / grid_node_size;
                    let shift = (normalized - Vector3::repeat(0.5)).map(T::floor);
                    let shifted = normalized - shift;

                    debug_assert!(check_shifted_quadratic(shifted));

                    let [x_weights, y_weights, z_weights]: [[T; KERNEL_QUADRATIC_LENGTH]; 3] = {
                        [
                            from_fn(|i| kernel_quadratic(shifted.x - i as T)),
                            from_fn(|i| kernel_quadratic(shifted.y - i as T)),
                            from_fn(|i| kernel_quadratic(shifted.z - i as T)),
                        ]
                    };

                    for (i, x_weight) in x_weights.iter().enumerate() {
                        for (j, y_weight) in y_weights.iter().enumerate() {
                            for (k, z_weight) in z_weights.iter().enumerate() {
                                let weight = x_weight * y_weight * z_weight;
                                let grid_idx = shift.map(|x| x as i32)
                                    + Vector3::new(i as i32, j as i32, k as i32);

                                let incompatibility = self
                                    .grid_collider_distances
                                    .get(&grid_idx)
                                    .and_then(|grid_node| {
                                        find_worst_incompatibility(
                                            collider_inside,
                                            &grid_node.lock(),
                                        )
                                    });
                                let grid_node_position = grid_idx.map(|i| i as T) * grid_node_size;
                                let to_grid_node = grid_node_position - position;

                                let grid = if let Some(collider_idx) = incompatibility {
                                    &self.grid_collider_momentums[collider_idx]
                                } else {
                                    &self.grid_momentum
                                };

                                let grid_idx = grid.map.get(&grid_idx).expect("missing node");
                                let grid_velocity = grid.velocities[*grid_idx];
                                *velocity += grid_velocity * weight;
                                *velocity_gradient +=
                                    (grid_velocity * weight) * to_grid_node.transpose();
                            }
                        }
                    }

                    *velocity_gradient *= 4. / grid_node_size / grid_node_size;
                },
            );

        Ok(self)
    }
}
