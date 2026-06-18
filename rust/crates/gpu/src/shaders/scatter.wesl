// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, NodeIdsAndColliderBits, get_global_index, div_ceil, murmur_of_cell, block_offset, kernel_quadratic, first_piola_stress_neo_hookean, first_piola_stress_inviscid, ParticleParameters};

@group(0) @binding(0)
var<storage, read> indirect: array<Indirect>;

@group(0) @binding(1)
var<storage, read_write> contributor_offsets: array<u32>;

@group(0) @binding(2)
var<storage, read_write> contributors: array<u32>;

@group(0) @binding(3)
var<storage, read_write> node_ids_and_collider_bits: array<NodeIdsAndColliderBits>;

@group(0) @binding(4)
var<storage, read_write> particle_tmp: array<mat4x4f>;

@group(0) @binding(5)
var<storage, read_write> node_momentums: array<vec4f>;

override WORKGROUP_SIZE: u32;
override GRID_NODE_SIZE: f32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= indirect.len {
        return;
    }

    // TODO
}
