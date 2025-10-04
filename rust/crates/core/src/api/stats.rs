use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Stats {
    pub total_particle_count: usize,
    pub total_grid_node_count: usize,
    // TODO:
    // per object count
    // time remaining to completion
    // time per frame
    // substeps per frame? or dt
    // memory usage
    // total disc usage
    // disc usage per frame
    // elastic and kinetic energy?
}
