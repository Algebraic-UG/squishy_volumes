// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Stats {
    pub state: StateStats,
    pub compute: Option<ComputeStats>,
    pub bytes_on_disk: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StateStats {
    pub total_particle_count: usize,
    pub per_object_count: BTreeMap<String, usize>,
    pub grid_node_count: Option<usize>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ComputeStats {
    pub remaining_time_sec: f32,
    pub last_frame_time_sec: f32,
    pub last_frame_substeps: usize,
}
