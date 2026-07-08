// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix3, Vector3};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use squishy_volumes_file_frame::ParticleFlags;
use squishy_volumes_util::profile;
use std::array::from_fn;

use super::*;

impl CpuState {
    // Update the particles' velocity and velocity gradients to be transported.
    pub fn collect_velocity(&mut self, grid_node_size: f32) {
        profile!("collect_velocity");
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
                    *velocity = Vector3::zeros();
                    *velocity_gradient = Matrix3::zeros();

                    let normalized = position / grid_node_size;
                    let shift = (normalized - Vector3::repeat(0.5)).map(f32::floor);
                    let shifted = normalized - shift;

                    let [x_weights, y_weights, z_weights]: [[f32; KERNEL_QUADRATIC_LENGTH]; 3] = {
                        [
                            from_fn(|i| kernel_quadratic(shifted.x - i as f32)),
                            from_fn(|i| kernel_quadratic(shifted.y - i as f32)),
                            from_fn(|i| kernel_quadratic(shifted.z - i as f32)),
                        ]
                    };

                    for (i, x_weight) in x_weights.iter().enumerate() {
                        for (j, y_weight) in y_weights.iter().enumerate() {
                            for (k, z_weight) in z_weights.iter().enumerate() {
                                let weight = x_weight * y_weight * z_weight;
                                let node_id = shift.map(|x| x as i32)
                                    + Vector3::new(i as i32, j as i32, k as i32);

                                let grid_node_position = node_id.map(|i| i as f32) * grid_node_size;
                                let to_grid_node = grid_node_position - position;

                                let grid_key = GridKey {
                                    node_id,
                                    collider_bits,
                                };

                                let grid_index =
                                    self.grid_nodes.map.get(&grid_key).expect("missing node");
                                let grid_velocity =
                                    self.grid_nodes.velocities[*grid_index as usize];
                                *velocity += grid_velocity * weight;
                                *velocity_gradient +=
                                    (grid_velocity * weight) * to_grid_node.transpose();
                            }
                        }
                    }

                    *velocity_gradient *= 4. / grid_node_size / grid_node_size;
                },
            );
    }
}
