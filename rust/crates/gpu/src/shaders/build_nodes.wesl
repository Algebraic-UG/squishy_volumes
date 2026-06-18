// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{PositionAndColliderBits, NodeIdAndColliderBits, get_global_index, node_offset, position_to_low_node};

@group(0) @binding(0)
var<storage, read_write> positions_and_collider_bits: array<PositionAndColliderBits>;

@group(0) @binding(1)
var<storage, read_write> owns: array<u32>;

@group(0) @binding(2)
var<storage, read_write> offsets: array<u32>;

@group(0) @binding(3)
var<storage, read_write> node_ids_and_collider_bits: array<NodeIdAndColliderBits>;

override WORKGROUP_SIZE: u32;
override GRID_NODE_SIZE: f32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    var global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= arrayLength(&positions_and_collider_bits) {
        return;
    }

    let position_and_collider_bits = positions_and_collider_bits[global_index];
    let own = owns[global_index];
    var node_index = offsets[global_index];

    let low_node_id = position_to_low_node(GRID_NODE_SIZE, position_and_collider_bits.position);

    for (var node = 0u; node < 27; node++) {
        if (own & (1u << node)) == 0 {
            continue;
        }

        node_ids_and_collider_bits[node_index].node_id = low_node_id + node_offset(node);
        node_ids_and_collider_bits[node_index].collider_bits = position_and_collider_bits.collider_bits;

        node_index += 1;
    }
}
