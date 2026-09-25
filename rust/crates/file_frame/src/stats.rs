// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Stats {
    pub state: StateStats,
    pub compute: Option<ComputeStats>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct StateStats {
    pub total_particle_count: u32,
    pub per_object_count: std::collections::BTreeMap<String, u32>,
    pub grid_node_count: Option<u32>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ComputeStats {
    pub remaining_time_sec: f32,
    pub last_frame_time_sec: f32,
    pub last_frame_substeps: u32,
}
