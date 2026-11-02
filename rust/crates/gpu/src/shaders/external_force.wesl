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

import super::util::{get_global_index, PositionAndColliderBits, HAS_GOAL, TOMBSTONED, NORMALIZATION_EPS, AnimatedGlobals};

@group(0) @binding(0)
var<storage, read_write> time_step: f32;

@group(0) @binding(1)
var<storage, read_write> globals: AnimatedGlobals;

@group(0) @binding(2)
var<storage, read_write> particle_flags: array<u32>;

@group(0) @binding(3)
var<storage, read_write> particle_positions_and_collider_bits: array<PositionAndColliderBits>;

@group(0) @binding(4)
var<storage, read_write> particle_velocities: array<vec3f>;

@group(0) @binding(5)
var<storage, read_write> particle_goals: array<vec3f>;

@group(0) @binding(6)
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

    var velocity = particle_velocities[global_index];

    velocity -= time_step * globals.damping * velocity;
    velocity += time_step * vec3f(globals.gravity_x, globals.gravity_y, globals.gravity_z);

    if (flags & HAS_GOAL) != 0 {
        let goal_position = particle_goals[global_index];
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

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
