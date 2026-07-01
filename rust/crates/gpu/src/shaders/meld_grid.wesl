// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, NodeIdAndColliderBits, get_global_index, murmur_of_node_id, TABLE_TRIES_EXCEEDED, TABLE_ENTRY_MISSING};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> node_ids_and_collider_bits: array<NodeIdAndColliderBits>;

@group(0) @binding(2)
var<storage, read_write> hash_table_multi: array<u32>;

@group(0) @binding(3)
var<storage, read_write> multi_offsets: array<u32>;

@group(0) @binding(4)
var<storage, read_write> multi: array<u32>;

@group(0) @binding(5)
var<storage, read_write> node_momentums_in: array<vec4f>;

@group(0) @binding(6)
var<storage, read_write> node_momentums_out: array<vec4f>;

@group(0) @binding(7)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    if is_error() {
        return;
    }

    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index == 0 {
        set_shader_id();
    }
    if global_index >= indirect.len {
        return;
    }
    // table length must be a power of two
    let table_mask = arrayLength(&hash_table_multi) - 1;

    let node_id_and_collider_bits = node_ids_and_collider_bits[global_index];
    let node_id = node_id_and_collider_bits.node_id;
    let collider_bits = node_id_and_collider_bits.collider_bits;

    var tries = 0;
    let hash = murmur_of_node_id(node_id, 0);
    var slot = hash & table_mask;
    if hash_table_multi[slot] == 0 {
        atomicOr(&status, TABLE_ENTRY_MISSING);
        return;
    }
    var node_index = hash_table_multi[slot] - 1;
    loop {
        if tries > 50 {
            atomicOr(&status, TABLE_TRIES_EXCEEDED);
            return;
        }

        if all(node_id == node_ids_and_collider_bits[node_index].node_id) {
            break;
        }
        slot += 1;
        slot &= table_mask;
        tries += 1;

        if hash_table_multi[slot] == 0 {
            atomicOr(&status, TABLE_ENTRY_MISSING);
            return;
        }

        node_index = hash_table_multi[slot] - 1;
    }

    let multi_start = multi_offsets[node_index];
    var multi_end = indirect.len;
    if node_index + 1 < indirect.len {
        multi_end = multi_offsets[node_index + 1];
    }

    var node_momentum = node_momentums_in[global_index];
    for (var multi_index = multi_start; multi_index < multi_end; multi_index++) {
        let other_index = multi[multi_index];
        if global_index == other_index {
            continue;
        }

        if !compatible(
            collider_bits,
            node_ids_and_collider_bits[other_index].collider_bits,
        ) {
            continue;
        }

        node_momentum += node_momentums_in[other_index];
    }

    node_momentums_out[global_index] = node_momentum;
}

fn compatible(a: u32, b: u32) -> bool {
    let mask = (a & b) >> 16;
    let diff = a ^ b;
    return (mask & diff) == 0;
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
