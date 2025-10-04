use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub loaded_state: Option<StateStats>,
    pub compute: Option<ComputeStats>,
    pub bytes_on_disk: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StateStats {
    pub total_particle_count: usize,
    pub total_grid_node_count: usize,
    pub per_object_count: BTreeMap<String, usize>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ComputeStats {
    pub remaining_time_sec: f32,
    pub last_frame_time_sec: f32,
    pub last_frame_substeps: usize,
}
