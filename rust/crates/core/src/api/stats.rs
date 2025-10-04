use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Stats {
    // these can be calculated from the state
    pub total_particle_count: Option<usize>,
    pub total_grid_node_count: Option<usize>,
    pub per_object_count: BTreeMap<String, usize>,

    // these can be calculated in the compute thread
    pub remaining_time_sec: Option<f32>,
    pub last_frame_time_sec: Option<f32>,
    pub last_frame_substeps: Option<usize>,

    // can be copied from existing tracking
    pub bytes_on_disk: Option<u64>,
}
