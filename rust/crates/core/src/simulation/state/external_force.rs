// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

use super::{PhaseInput, State, profile};

impl State {
    pub(super) fn external_force(mut self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("external_force");
        let time_step = phase_input.time_step;
        let gravity = phase_input.setup.settings.gravity;
        // TODO: try chaining
        for grid in self.grid_momentums_mut() {
            grid.velocities
                .par_iter_mut()
                .for_each(|velocity| *velocity += gravity * time_step);
        }

        Ok(self)
    }
}
