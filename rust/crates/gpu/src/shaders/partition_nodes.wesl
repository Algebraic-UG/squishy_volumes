// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

// Inspired by https://nosferalatu.com/SimpleGPUHashTable.html

import super::util::{PositionAndColliderBits, get_global_index, node_offset, position_to_low_node, murmur_of_node_id};

@group(0) @binding(0)
var<storage, read_write> positions_and_collider_bits: array<PositionAndColliderBits>;

@group(0) @binding(1)
var<storage, read_write> owns: array<u32>;

@group(0) @binding(2)
var<storage, read_write> hash_table: array<atomic<u32>>;

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
    let collider_bits = position_and_collider_bits.collider_bits;
    let low_node_id = position_to_low_node(GRID_NODE_SIZE, position_and_collider_bits.position);

    // table length must be a power of two
    let table_mask = arrayLength(&hash_table) - 1;
    let index_mask = (1u << 27) - 1;
    var own = 0u;
    for (var node = 0u; node < 27; node++) {
        let node_id = low_node_id + node_offset(node);
        let node_and_index = (node << 27) | (global_index + 1);

        let hash = murmur_of_node_id(node_id, collider_bits);

        var slot = hash & table_mask;
        loop {
            let result = atomicCompareExchangeWeak(&hash_table[slot], 0, node_and_index);

            if result.exchanged {
                own |= 1u << node;
                break;
            }

            let old_node: u32 = result.old_value >> 27;

            let old_index = (result.old_value & index_mask) - 1;
            let old_position_and_collider_bits = positions_and_collider_bits[old_index];
            if old_position_and_collider_bits.collider_bits != position_and_collider_bits.collider_bits {
                continue;
            }

            let old_node_id = position_to_low_node(GRID_NODE_SIZE, old_position_and_collider_bits.position) + node_offset(old_node);
            if all(old_node_id == node_id) {
                break;
            }

            continuing {
                slot += 1;
                slot &= table_mask;
            }
        }
    }

    owns[global_index] = own;
}
