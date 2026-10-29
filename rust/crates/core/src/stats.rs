// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use squishy_volumes_cache::{ComputeStats, Stats};
use squishy_volumes_file_frame::IoState;

// particle counts are not part of the stats, they are in the input header
pub fn make_stats(io_state: &IoState, compute: Option<ComputeStats>) -> Stats {
    Stats {
        grid_node_count: io_state
            .grid_nodes
            .as_ref()
            .map(|grid_nodes| grid_nodes.collider_bits.len() as u32),
        compute,
    }
}
