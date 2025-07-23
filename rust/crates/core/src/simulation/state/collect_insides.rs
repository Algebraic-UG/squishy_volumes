// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use blended_mpm_api::T;
use fxhash::FxHashMap;
use itertools::izip;
use nalgebra::{Matrix4, Vector3, Vector4};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{array::from_fn, collections::hash_map::Entry};

use crate::{
    math::{NORMALIZATION_EPS, safe_inverse::SafeInverse},
    simulation::weights::kernel_quadratic,
    weights::KERNEL_QUADRATIC_LENGTH,
};

use super::{PhaseInput, State, check_shifted_quadratic, profile};

impl State {
    // Collect the splatted distance information from the grid to the particles.
    // The particles just store whether they are inside.
    // While the information is at hand, perform penalty velocity updates for penetration.
    pub(super) fn collect_insides(mut self, phase_input: PhaseInput) -> Result<Self> {
        profile!("collect_insides");

        let time_step = phase_input.time_step;
        let grid_node_size = phase_input.setup.settings.grid_node_size;

        // Since the grid has only partial information about the distances,
        // we need to do MLS interpolation.
        struct DistanceHelper {
            distance_and_gradient: Vector4<T>,
            matrix: Matrix4<T>,
        }

        impl Default for DistanceHelper {
            fn default() -> Self {
                Self {
                    distance_and_gradient: Vector4::zeros(),
                    matrix: Matrix4::zeros(),
                }
            }
        }

        izip!(
            self.particles.positions.iter(),
            self.particles.velocities.iter_mut(),
            self.particles.collider_insides.iter_mut(),
        )
        .par_bridge()
        .for_each(|(position, velocity, collider_inside)| {
            let mut distance_helpers: FxHashMap<usize, DistanceHelper> = Default::default();

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
                        let grid_idx =
                            shift.map(|x| x as i32) + Vector3::new(i as i32, j as i32, k as i32);

                        let Some(grid_node) = self.grid_collider_distances.get(&grid_idx) else {
                            continue;
                        };

                        let linear_basis = Vector4::new(
                            1.,
                            i as T - shifted.x,
                            j as T - shifted.y,
                            k as T - shifted.z,
                        );
                        for (collider_idx, weighted_distance) in
                            grid_node.lock().weighted_distances.iter()
                        {
                            let distance_helper =
                                distance_helpers.entry(*collider_idx).or_default();
                            distance_helper.distance_and_gradient +=
                                linear_basis * weighted_distance.distance * weight;
                            distance_helper.matrix +=
                                (linear_basis * weight) * linear_basis.transpose();
                        }
                    }
                }
            }

            // Convert the collected information into signed distance and normal.
            // We need to be sure that the collected information is reliable.
            // It's better to have a particle be oblivious to a collider for longer
            // than to accept wonky distance and normal.
            distance_helpers.retain(
                |_,
                 DistanceHelper {
                     distance_and_gradient,
                     matrix,
                 }| {
                    let Some(m_inv) = matrix.safe_inverse() else {
                        return false;
                    };
                    *distance_and_gradient = m_inv * *distance_and_gradient;
                    let Some(gradient) = Vector3::new(
                        distance_and_gradient.y,
                        distance_and_gradient.z,
                        distance_and_gradient.w,
                    )
                    .try_normalize(NORMALIZATION_EPS) else {
                        return false;
                    };
                    distance_and_gradient.y = gradient.x;
                    distance_and_gradient.z = gradient.y;
                    distance_and_gradient.w = gradient.z;
                    true
                },
            );

            // Now actually update the bits.
            // If the particle moved away from a collider it drops the info.
            collider_inside.retain(|collider_idx, _| distance_helpers.contains_key(collider_idx));
            for (collider_idx, distance_helper) in distance_helpers.into_iter() {
                let distance = distance_helper.distance_and_gradient.x;
                let normal = Vector3::new(
                    distance_helper.distance_and_gradient.y,
                    distance_helper.distance_and_gradient.z,
                    distance_helper.distance_and_gradient.w,
                );
                match collider_inside.entry(collider_idx) {
                    // We already know the collider.
                    // Stick with the side and receive penetration penalty.
                    Entry::Occupied(occupied_entry) => {
                        if occupied_entry.get() ^ (distance < 0.) {
                            *velocity -= normal * normal.dot(velocity);
                            *velocity += normal
                                * (self.collider_objects[collider_idx]
                                    .kinematic
                                    .point_velocity_from_world(*position)
                                    .dot(&normal)
                                    - distance / time_step);
                        }
                    }
                    // Collider is new, accept the side
                    Entry::Vacant(vacant_entry) => {
                        vacant_entry.insert(distance_helper.distance_and_gradient.x < 0.);
                    }
                }
            }
        });

        Ok(self)
    }
}
