// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{get_global_index, PositionAndColliderBits, HAS_GOAL, TOMBSTONED, NORMALIZATION_EPS, AnimatedGlobals};

@group(0) @binding(0)
var<storage, read_write> time: f32;

@group(0) @binding(1)
var<storage, read_write> time_step: f32;

@group(0) @binding(2)
var<storage, read_write> globals_start: AnimatedGlobals;

@group(0) @binding(3)
var<storage, read_write> globals_end: AnimatedGlobals;

@group(0) @binding(4)
var<storage, read_write> particle_flags: array<u32>;

@group(0) @binding(5)
var<storage, read_write> particle_positions_and_collider_bits: array<PositionAndColliderBits>;

@group(0) @binding(6)
var<storage, read_write> particle_velocities: array<vec3f>;

@group(0) @binding(7)
var<storage, read_write> particle_goals_start: array<vec3f>;

@group(0) @binding(8)
var<storage, read_write> particle_goals_end: array<vec3f>;

@group(0) @binding(9)
var<storage, read_write> status: atomic<u32>;

override FRAMES_PER_SECOND: u32;
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
    if global_index >= arrayLength(&particle_velocities) {
        return;
    }

    let flags = particle_flags[global_index];
    if (flags & TOMBSTONED) != 0 {
        return;
    }

    let factor = time * f32(FRAMES_PER_SECOND);
    let globals = interpolate_globals(factor);
    var velocity = particle_velocities[global_index];

    velocity -= time_step * globals.damping * velocity;
    velocity += time_step * vec3f(globals.gravity_x, globals.gravity_y, globals.gravity_z);

    if (flags & HAS_GOAL) != 0 {
        let goal_position = //
            (1. - factor) * particle_goals_start[global_index]
            + factor * particle_goals_end[global_index];
        let position = particle_positions_and_collider_bits[global_index].position;
        let to_goal = goal_position - position;
        let distance = length(to_goal);
        if distance > NORMALIZATION_EPS {
            let to_goal_dir = to_goal / distance;
            velocity += (globals.goal_stiffness * distance
                - globals.goal_damping * dot(to_goal_dir, velocity)) * time_step * to_goal_dir;
        }
    }

    particle_velocities[global_index] = velocity;
}

fn interpolate_globals(factor: f32) -> AnimatedGlobals {
    var result: AnimatedGlobals;
    result.gravity_x = (1. - factor) * globals_start.gravity_x + factor * globals_end.gravity_x;
    result.gravity_y = (1. - factor) * globals_start.gravity_y + factor * globals_end.gravity_y;
    result.gravity_z = (1. - factor) * globals_start.gravity_z + factor * globals_end.gravity_z;
    result.goal_stiffness = (1. - factor) * globals_start.goal_stiffness + factor * globals_end.goal_stiffness;
    result.goal_damping = (1. - factor) * globals_start.goal_damping + factor * globals_end.goal_damping;
    result.damping = (1. - factor) * globals_start.damping + factor * globals_end.damping;
    return result;
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
