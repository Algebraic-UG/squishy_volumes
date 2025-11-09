// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Context, Result};

use crate::{
    profile,
    simulation::kinematic::{Kinematic, ScriptedMovement},
};

use super::{PhaseInput, State};

impl State {
    pub(super) fn move_collider(mut self, _: &mut PhaseInput) -> Result<Self> {
        profile!("move_collider");
        for collider in &mut self.collider_objects {
            let Some((from, to)) =
                ScriptedMovement::find_iterpolation_pair(&collider.scripted_movements, self.time)
            else {
                collider.has_moved = false;
                continue;
            };
            let kinematic = Kinematic::interpolate(from, to, self.time)
                .context("Movement interpolation failed")?;
            collider.has_moved = collider.kinematic != kinematic;
            collider.kinematic = kinematic;
        }
        Ok(self)
    }
}
