// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;

use crate::profile;

use super::{PhaseInput, State};

impl State {
    pub fn interpolate_input(mut self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("interpolate_input");
        self.interpolated_input = Some(phase_input.input_interpolation.interpolate(self.time)?);
        Ok(self)
    }
}
