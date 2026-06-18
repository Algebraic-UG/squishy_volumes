// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

// Inspired by https://nosferalatu.com/SimpleGPUHashTable.html

import super::util::{Indirect, NodeIdAndColliderBits, get_global_index, murmur_of_node_id};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> node_ids_and_collider_bits: array<NodeIdAndColliderBits>;

@group(0) @binding(2)
var<storage, read_write> hash_table: array<atomic<u32>>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    var global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= indirect.len {
        return;
    }

    let node_id_and_collider_bits = node_ids_and_collider_bits[global_index];

    // table length must be a power of two
    let table_mask = arrayLength(&hash_table) - 1;
    let hash = murmur_of_node_id(node_id_and_collider_bits.node_id);
    var slot = hash & table_mask;

    while !atomicCompareExchangeWeak(&hash_table[slot], 0, global_index + 1).exchanged {
        slot += 1;
        slot &= table_mask;
    }
}
