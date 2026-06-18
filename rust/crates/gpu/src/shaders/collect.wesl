// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, PositionAndColliderBits, NodeIdsAndColliderBits, get_global_index, murmur_of_node_id};

@group(0) @binding(0)
var<storage, read_write> hash_table: array<u32>;

@group(0) @binding(1)
var<storage, read_write> node_ids_and_collider_bits: array<NodeIdsAndColliderBits>;

@group(0) @binding(2)
var<storage, read_write> node_momentums: array<vec4f>;

// particle stuff

@group(0) @binding(3)
var<storage, read_write> particle_positions_and_collider_bits: array<PositionAndColliderBits>;

@group(0) @binding(4)
var<storage, read_write> particle_position_gradients: array<mat3x3<f32>>;

@group(0) @binding(5)
var<storage, read_write> particle_velocities: array<vec3<f32>>;

@group(0) @binding(6)
var<storage, read_write> particle_velocity_gradients: array<mat3x3<f32>>;

override WORKGROUP_SIZE: u32;
override CELL_SIZE: f32;
override TIME_STEP: f32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let grid_node_size = CELL_SIZE * 0.5;
    let scaling = TIME_STEP * 4. / (grid_node_size * grid_node_size);

    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= arrayLength(&positions_and_collider_bits) {
        return;
    }

    // TODO
}
