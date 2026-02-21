// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;

use super::{PhaseInput, State};

impl State {
    pub fn implicit_solve(self, _phase_input: &mut PhaseInput) -> Result<Self> {
        panic!("This isn't ready yet :(");
    }
}
