// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, PositionAndColliderBits, NodeIdAndColliderBits, get_global_index, murmur_of_node_id, node_offset, position_to_low_node, kernel_quadratic};

@group(0) @binding(0)
var<storage, read_write> hash_table: array<u32>;

@group(0) @binding(1)
var<storage, read_write> node_ids_and_collider_bits: array<NodeIdAndColliderBits>;

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
override GRID_NODE_SIZE: f32;
override TIME_STEP: f32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {

    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= arrayLength(&particle_positions_and_collider_bits) {
        return;
    }

    let position_and_collider_bits = particle_positions_and_collider_bits[global_index];
    let low_node_id = position_to_low_node(GRID_NODE_SIZE, position_and_collider_bits.position);
    let normalized_position = position_and_collider_bits.position / GRID_NODE_SIZE;
    let collider_bits = position_and_collider_bits.collider_bits;

    var velocity = TIME_STEP * vec3f(0., 0., -9.8);
    //var velocity = vec3f(0.);
    var velocity_gradient = mat3x3f();

    // table length must be a power of two
    let table_mask = arrayLength(&hash_table) - 1;
    for (var node = 0u; node < 27; node++) {
        let node_id = low_node_id + node_offset(node);
        let hash = murmur_of_node_id(node_id, collider_bits);

        var slot = hash & table_mask;
        //  this must never be zero, or we're in big trouble
        var node_index = hash_table[slot] - 1;
        loop {
            let entry = node_ids_and_collider_bits[node_index];
            if entry.collider_bits == collider_bits && all(node_id == entry.node_id) {
                break;
            }

            slot += 1;
            slot &= table_mask;

            node_index = hash_table[slot] - 1;
        }

        let node_momentum = node_momentums[node_index];
        if node_momentum.w == 0. {
            continue;
        }

        let to_node_normalized = vec3f(node_id) - normalized_position;
        let weight: f32 = //
              kernel_quadratic(to_node_normalized.x)
            * kernel_quadratic(to_node_normalized.y)
            * kernel_quadratic(to_node_normalized.z);
        let tmp = node_momentum.xyz * (weight / node_momentum.w);

        velocity += tmp;
        velocity_gradient += mat3x3f(
            tmp * to_node_normalized.x * GRID_NODE_SIZE,
            tmp * to_node_normalized.y * GRID_NODE_SIZE,
            tmp * to_node_normalized.z * GRID_NODE_SIZE,
        );
    }

    velocity_gradient *= 4. / GRID_NODE_SIZE / GRID_NODE_SIZE;

    particle_positions_and_collider_bits[global_index].position += velocity * TIME_STEP;
    particle_position_gradients[global_index] += velocity_gradient * particle_position_gradients[global_index] * TIME_STEP;

    particle_velocities[global_index] = velocity;
    particle_velocity_gradients[global_index] = velocity_gradient;
}
