// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::BTreeMap;

use squishy_volumes_file_frame::{IoState, StateStats};
use squishy_volumes_file_input::InputObject;

pub fn make_state_stats(objects: &BTreeMap<String, InputObject>, io_state: &IoState) -> StateStats {
    let mut total_particle_count = 0;
    let per_object_count: BTreeMap<String, usize> = objects
        .iter()
        .filter_map(|(name, object)| {
            if let InputObject::Particles { num_particles } = object {
                total_particle_count += num_particles;
                Some((name.clone(), *num_particles))
            } else {
                None
            }
        })
        .collect();
    let grid_node_count = io_state
        .grid_nodes
        .as_ref()
        .map(|grid_nodes| grid_nodes.collider_bits.len());

    StateStats {
        total_particle_count,
        per_object_count,
        grid_node_count,
    }
}
