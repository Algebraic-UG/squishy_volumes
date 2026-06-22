// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, NodeIdAndColliderBits, get_global_index, murmur_of_node_id};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> hash_table: array<u32>;

@group(0) @binding(2)
var<storage, read_write> node_ids_and_collider_bits: array<NodeIdAndColliderBits>;

@group(0) @binding(3)
var<storage, read_write> node_momentums_in: array<vec4f>;

@group(0) @binding(4)
var<storage, read_write> node_momentums_out: array<vec4f>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {

    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= indirect.len {
        return;
    }
    // table length must be a power of two
    let table_mask = arrayLength(&hash_table) - 1;

    let node_id_and_collider_bits = node_ids_and_collider_bits[global_index];
    let hash = murmur_of_node_id(node_id_and_collider_bits.node_id, 0);

    var node_momentum = vec4f(0.);

    var slot = hash & table_mask;
    loop {
        var node_index = hash_table[slot];
        if node_index == 0 {
            break;
        }
        node_index -= 1;

        let other_node_id_and_collider_bits = node_ids_and_collider_bits[node_index];

        let comapatible = compatible(
            node_id_and_collider_bits.collider_bits,
            other_node_id_and_collider_bits.collider_bits,
        );
        let same_position = all(
            node_id_and_collider_bits.node_id ==
            other_node_id_and_collider_bits.node_id
        );

        if comapatible && same_position {
            node_momentum += node_momentums_in[node_index];
        }

        slot += 1;
        slot &= table_mask;
    }

    node_momentums_out[global_index] = node_momentum;
}

fn compatible(a: u32, b: u32) -> bool {
    let mask = (a & b) >> 16;
    let diff = a ^ b;
    return (mask & diff) == 0;
}
