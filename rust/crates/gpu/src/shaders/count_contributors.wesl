// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

// Inspired by https://nosferalatu.com/SimpleGPUHashTable.html

import super::util::{Indirect, PositionAndColliderBits, NodeIdAndColliderBits, get_global_index, murmur_of_node_id, node_offset, position_to_low_node, TABLE_TRIES_EXCEEDED, TABLE_ENTRY_MISSING};

@group(0) @binding(0)
var<storage, read_write> positions_and_collider_bits: array<PositionAndColliderBits>;

@group(0) @binding(1)
var<storage, read_write> hash_table: array<u32>;

@group(0) @binding(2)
var<storage, read_write> node_ids_and_collider_bits: array<NodeIdAndColliderBits>;

@group(0) @binding(3)
var<storage, read_write> contributor_counts: array<atomic<u32>>;

@group(0) @binding(4)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;
override GRID_NODE_SIZE: f32;
override TABLE_TRIES: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    if is_error() {
        return;
    }
    var global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index == 0 {
        set_shader_id();
    }
    if global_index >= arrayLength(&positions_and_collider_bits) {
        return;
    }

    let position_and_collider_bits = positions_and_collider_bits[global_index];
    let low_node_id = position_to_low_node(GRID_NODE_SIZE, position_and_collider_bits.position);
    let collider_bits = position_and_collider_bits.collider_bits;

    // table length must be a power of two
    let table_mask = arrayLength(&hash_table) - 1;

    for (var node = 0u; node < 27; node++) {
        let node_id = low_node_id + node_offset(node);
        let hash = murmur_of_node_id(node_id, collider_bits);

        var tries = 0u;
        var slot = hash & table_mask;
        if hash_table[slot] == 0 {
            atomicOr(&status, TABLE_ENTRY_MISSING);
            return;
        }
        var node_index = hash_table[slot] - 1;
        loop {
            if tries > TABLE_TRIES {
                atomicOr(&status, TABLE_TRIES_EXCEEDED);
                return;
            }

            let entry = node_ids_and_collider_bits[node_index];
            if entry.collider_bits == collider_bits && all(node_id == entry.node_id) {
                break;
            }

            slot += 1;
            slot &= table_mask;
            tries += 1;

            if hash_table[slot] == 0 {
                atomicOr(&status, TABLE_ENTRY_MISSING);
                return;
            }

            node_index = hash_table[slot] - 1;
        }

        atomicAdd(&contributor_counts[node_index], 1);
    }
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
