// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index, murmur_of_node_id, NodeIdAndColliderBits};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> node_ids_and_collider_bits: array<NodeIdAndColliderBits>;

@group(0) @binding(2)
var<storage, read_write> hashes_node_ids: array<u32>;

@group(0) @binding(3)
var<storage, read_write> hashes_node_ids_and_collider_bits: array<u32>;

@group(0) @binding(4)
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

    let node_id_and_collider_bits = node_ids_and_collider_bits[global_index];
    hashes_node_ids[global_index] = murmur_of_node_id(node_id_and_collider_bits.node_id, 0);
    hashes_node_ids_and_collider_bits[global_index] = murmur_of_node_id(
        node_id_and_collider_bits.node_id,
        node_id_and_collider_bits.collider_bits,
    );
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
