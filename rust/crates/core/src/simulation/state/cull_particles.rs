// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use rayon::{Scope, scope};

use crate::simulation::particles::Particles;

use super::{PhaseInput, State, profile};

impl State {
    pub(super) fn cull_particles(mut self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("cull_particles");
        let within_bounds: Vec<_> = self
            .particles
            .positions
            .iter()
            .map(|position| {
                position
                    .zip_zip_map(
                        &phase_input.setup.settings.domain_min,
                        &phase_input.setup.settings.domain_max,
                        |p, min, max| p > min && p < max,
                    )
                    .iter()
                    .all(|b| *b)
            })
            .collect();

        // maybe there'll be more reasons to cull a particle
        let keep = within_bounds.as_slice();

        fn cull<'a, T: Clone + Send>(s: &Scope<'a>, keep: &'a [bool], to_cull: &'a mut Vec<T>) {
            s.spawn(move |_| {
                let mut i = 0;
                to_cull.retain(|_| {
                    let keep = keep[i];
                    i += 1;
                    keep
                });
            });
        }

        let Particles {
            sort_map,
            reverse_sort_map,
            parameters,
            masses,
            initial_volumes,
            initial_positions,
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
            elastic_energies,
            collider_insides,
            trial_position_gradients,
            action_matrices,
        } = &mut self.particles;

        scope(|s| {
            cull(s, keep, sort_map);
            cull(s, keep, reverse_sort_map);
            cull(s, keep, parameters);
            cull(s, keep, masses);
            cull(s, keep, initial_volumes);
            cull(s, keep, initial_positions);
            cull(s, keep, positions);
            cull(s, keep, position_gradients);
            cull(s, keep, velocities);
            cull(s, keep, velocity_gradients);
            cull(s, keep, elastic_energies);
            cull(s, keep, collider_insides);
            cull(s, keep, trial_position_gradients);
            cull(s, keep, action_matrices);
        });

        Ok(self)
    }
}
