// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{get_global_index, PositionAndColliderBits, HAS_GOAL};

@group(0) @binding(0)
var<storage, read_write> gravity: vec3f;

@group(0) @binding(1)
var<storage, read_write> particle_flags: array<u32>;

@group(0) @binding(2)
var<storage, read_write> particle_positions_and_collider_bits: array<PositionAndColliderBits>;

@group(0) @binding(3)
var<storage, read_write> particle_velocities: array<vec3f>;

@group(0) @binding(4)
var<storage, read_write> particle_goals_start: array<vec3f>;

@group(0) @binding(5)
var<storage, read_write> particle_goals_end: array<vec3f>;

@group(0) @binding(6)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;
override TIME_STEP: f32;

var<immediate> factor: f32;

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
    if global_index >= arrayLength(&particle_velocities) {
        return;
    }

    if (particle_flags[global_index] & HAS_GOAL) != 0 {
        let goal_position = //
            (1. - factor) * particle_goals_start[global_index]
            + factor * particle_goals_end[global_index];
        let position = particle_positions_and_collider_bits[global_index].position;
        particle_velocities[global_index] = (goal_position - position) / TIME_STEP;
        return;
    }

    particle_velocities[global_index] += TIME_STEP * gravity;
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
