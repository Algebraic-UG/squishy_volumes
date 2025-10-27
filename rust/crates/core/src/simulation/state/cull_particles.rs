// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::simulation::particles::ParticleState;

use super::{PhaseInput, State, profile};

impl State {
    pub(super) fn cull_particles(mut self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("cull_particles");
        self.particles
            .states
            .par_iter_mut()
            .zip(&self.particles.positions)
            .for_each(|(state, position)| {
                if *state == ParticleState::Tombstoned {
                    return;
                }

                let within_bounds = position
                    .zip_zip_map(
                        &phase_input.setup.settings.domain_min,
                        &phase_input.setup.settings.domain_max,
                        |p, min, max| p > min && p < max,
                    )
                    .iter()
                    .all(|b| *b);
                if within_bounds {
                    return;
                }

                *state = ParticleState::Tombstoned;
            });

        Ok(self)
    }
}
