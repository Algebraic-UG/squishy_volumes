// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct ObjectParticles {
    pub particles: Vec<usize>,
}
