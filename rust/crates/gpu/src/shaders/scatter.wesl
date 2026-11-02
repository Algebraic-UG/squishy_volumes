// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

import super::util::{Indirect, NodeIdAndColliderBits, get_global_index, div_ceil, murmur_of_cell, block_offset, kernel_quadratic, first_piola_stress_neo_hookean, first_piola_stress_inviscid};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> total_contributors: u32;

@group(0) @binding(2)
var<storage, read_write> contributor_offsets: array<u32>;

@group(0) @binding(3)
var<storage, read_write> contributors: array<u32>;

@group(0) @binding(4)
var<storage, read_write> node_ids_and_collider_bits: array<NodeIdAndColliderBits>;

@group(0) @binding(5)
var<storage, read_write> particle_tmp: array<mat4x4f>;

@group(0) @binding(6)
var<storage, read_write> node_momentums: array<vec4f>;

@group(0) @binding(7)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;
override GRID_NODE_SIZE: f32;

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

    let contributors_start = contributor_offsets[global_index];
    var contributors_end = total_contributors;
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

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
