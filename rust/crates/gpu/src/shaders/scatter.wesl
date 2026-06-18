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

    let contributors_start = contributor_offsets[global_index];
    var contributors_end = arrayLength(&contributors);
    if global_index + 1 < indirect.len {
        contributors_end = contributor_offsets[global_index + 1];
    }

    let normalized_node_position = vec3f(node_ids_and_collider_bits[global_index].node_id);

    var node_momentum = vec4f(0.);
    for (var contributor = contributors_start; contributor < contributors_end; contributor++) {
        let tmp = particle_tmp[contributors[contributor]];
        let matrix_part = mat3x3f(tmp[0].xyz, tmp[1].xyz, tmp[2].xyz);
        let vector_part = tmp[3].xyz;
        let mass = tmp[3].w;
        let normalized_particle_position = vec3f(tmp[0].w, tmp[1].w, tmp[2].w);

        let to_node_normalized = normalized_node_position - normalized_particle_position;
        let weight: f32 = //
              kernel_quadratic(to_node_normalized.x)
            * kernel_quadratic(to_node_normalized.y)
            * kernel_quadratic(to_node_normalized.z);

        node_momentum += vec4f(vector_part + matrix_part * to_node_normalized * GRID_NODE_SIZE, mass) * weight;
    }
    node_momentums[global_index] = node_momentum;
}
