// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

// Inspired by https://nosferalatu.com/SimpleGPUHashTable.html

import super::util::{Indirect, PositionAndColliderBits, NodeIdAndColliderBits, get_global_index, murmur_of_node_id, node_offset, position_to_low_node};

@group(0) @binding(0)
var<storage, read_write> positions_and_collider_bits: array<PositionAndColliderBits>;

@group(0) @binding(1)
var<storage, read_write> hash_table: array<u32>;

@group(0) @binding(2)
var<storage, read_write> node_ids_and_collider_bits: array<NodeIdAndColliderBits>;

@group(0) @binding(3)
var<storage, read_write> contributor_counts: array<atomic<u32>>;

@group(0) @binding(4)
var<storage, read_write> contributor_offsets: array<u32>;

@group(0) @binding(5)
var<storage, read_write> contributors: array<u32>;

override WORKGROUP_SIZE: u32;
override GRID_NODE_SIZE: f32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    var global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= arrayLength(&positions_and_collider) {
        return;
    }

    let position_and_collider_bits = positions_and_collider_bits[global_index];
    let low_node_id = position_to_low_node(GRID_NODE_SIZE, position_and_collider_bits.position);
    let collider_bits = position_and_collider_bits.collider_bits;

    // table length must be a power of two
    let table_mask = arrayLength(&hash_table) - 1;

    for (var node = 0u; node < 27; node++) {
        let node_id = low_node_id + node_offset(node);
        let hash = murmur_of_node_id(node_id_and_collider_bits.node_id);

        var slot = hash & table_mask;
        //  this must never be zero, or we're in big trouble
        var node_index = hash_table[slot] - 1;
        loop {
            let entry = node_ids_and_collider_bits[node_index];
            if entry.collider_bits == collider_bits && all(node_id == entry.node_id) {
                break;
            }

            slot += 1;
            slot &= table_mask;

            node_index = hash_table[slot] - 1;
        }

        let local_offset = atomicAdd(&contributor_counts[node_index], 1);
        let index = local_offset + contributor_offsets[node_index];

        contributors[index] = global_index;
    }
}
