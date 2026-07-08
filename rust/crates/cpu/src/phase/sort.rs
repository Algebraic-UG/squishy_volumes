// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use rayon::{Scope, scope, slice::ParallelSliceMut};
use squishy_volumes_util::profile;

use super::*;

impl CpuState {
    // This is only to optimize memory access.
    pub fn sort(&mut self, grid_node_size: f32) -> Result<(), Error> {
        profile!("sort");

        // Probably many other alternatives exist, e.g. one could do a z-order curve.
        // This seemed to be faster though. Maybe try again with cached keys?
        #[derive(Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
        struct SortingPos {
            i: i32,
            j: i32,
            k: i32,
        }

        let to_sorting_pos = |position: &Vector3<f32>| SortingPos {
            i: (position.x / grid_node_size - 0.5).floor() as i32,
            j: (position.y / grid_node_size - 0.5).floor() as i32,
            k: (position.z / grid_node_size - 0.5).floor() as i32,
        };

        {
            profile!("simulated particles");

            let mut tmp: Vec<(usize, SortingPos)> = {
                profile!("create index-position-pairs");
                self.particles
                    .positions
                    .iter()
                    .map(to_sorting_pos)
                    .enumerate()
                    .collect()
            };

            {
                profile!("actual sorting");
                tmp.par_sort_unstable_by_key(|pair| pair.1);
            }

            let permutation: Vec<usize> = {
                profile!("unzip");
                tmp.into_iter().map(|(idx, _)| idx).collect()
            };

            {
                profile!("apply permutation");
                let Particles {
                    // These need to be moved with the particles
                    flags,
                    positions,
                    initial_positions,
                    sort_map,
                    parameters,
                    position_gradients,
                    velocities,
                    velocity_gradients,
                    collider_bits,

                    // These will be overwritten anyway
                    reverse_sort_map: _,
                    elastic_energies: _,
                } = &mut self.particles;

                fn permute<'a, T: Clone + Send>(
                    s: &Scope<'a>,
                    permutation: &'a [usize],
                    to_permute: &'a mut Vec<T>,
                ) {
                    s.spawn(move |_| {
                        let lookup = to_permute.clone();
                        assert!(permutation.len() == to_permute.len());
                        for (&prior_position, to_permute) in permutation.iter().zip(to_permute) {
                            *to_permute = lookup[prior_position].clone();
                        }
                    });
                }

                scope(|s| {
                    permute(s, &permutation, flags);
                    permute(s, &permutation, positions);
                    permute(s, &permutation, initial_positions);
                    permute(s, &permutation, sort_map);
                    permute(s, &permutation, parameters);
                    permute(s, &permutation, position_gradients);
                    permute(s, &permutation, velocities);
                    permute(s, &permutation, velocity_gradients);
                    permute(s, &permutation, collider_bits);
                });
            }

            {
                profile!("reverse sort map");
                self.particles
                    .reverse_sort_map
                    .resize(self.particles.sort_map.len(), 0);
                for (current, original) in self.particles.sort_map.iter().enumerate() {
                    self.particles.reverse_sort_map[*original as usize] = current as u32;
                }
            }
        }

        Ok(())
    }
}
