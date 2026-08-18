// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{get_global_index, PositionAndColliderBits, TOMBSTONED};

@group(0) @binding(0)
var<storage, read_write> particle_flags: array<u32>;

@group(0) @binding(1)
var<storage, read_write> particle_positions_and_collider_bits: array<PositionAndColliderBits>;

@group(0) @binding(2)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;

override DOMAIN_MIN_X: f32;
override DOMAIN_MIN_Y: f32;
override DOMAIN_MIN_Z: f32;
override DOMAIN_MAX_X: f32;
override DOMAIN_MAX_Y: f32;
override DOMAIN_MAX_Z: f32;

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
    if global_index >= arrayLength(&particle_flags) {
        return;
    }

    var flags = particle_flags[global_index];
    if (flags & TOMBSTONED) != 0 {
        return;
    }

    let position = particle_positions_and_collider_bits[global_index].position;
    let domain_min = vec3f(DOMAIN_MIN_X, DOMAIN_MIN_Y, DOMAIN_MIN_Z);
    let domain_max = vec3f(DOMAIN_MAX_X, DOMAIN_MAX_Y, DOMAIN_MAX_Z);
    if any(position < domain_min) || any(position > domain_max) {
        flags |= TOMBSTONED;
        particle_flags[global_index] = flags;
    }
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
