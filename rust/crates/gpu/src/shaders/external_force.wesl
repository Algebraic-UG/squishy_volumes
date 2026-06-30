// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::get_global_index;

@group(0) @binding(0)
var<storage, read_write> gravity: vec3f;

@group(0) @binding(1)
var<storage, read_write> particle_velocities: array<vec3f>;

@group(0) @binding(2)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override TIME_STEP: f32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    if atomicLoad(&status) != 0 {
        return;
    }
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= arrayLength(&particle_velocities) {
        return;
    }

    particle_velocities[global_index] += TIME_STEP * gravity;
}
