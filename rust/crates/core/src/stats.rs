// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::BTreeMap;

use squishy_volumes_file_frame::{IoState, StateStats};
use squishy_volumes_file_input::{Header, Object};

pub fn make_state_stats(header: &Header, io_state: &IoState) -> StateStats {
    let per_object_count: BTreeMap<String, u32> = header
        .objects
        .iter()
        .filter_map(|(name, object)| {
            if let Object::Particles(object) = object {
                Some((name.clone(), object.particle_range.len() as u32))
            } else {
                None
            }
        })
        .collect();
    let grid_node_count = io_state
        .grid_nodes
        .as_ref()
        .map(|grid_nodes| grid_nodes.collider_bits.len() as u32);

    StateStats {
        total_particle_count: header.total_particles as u32,
        per_object_count,
        grid_node_count,
    }
}
