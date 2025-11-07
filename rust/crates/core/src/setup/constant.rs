// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SetupConstant {
    // for input and output frames
    pub frames_per_second: u32,
    pub objects: Vec<String>,
}
