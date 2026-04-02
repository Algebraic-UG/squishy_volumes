// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

//https://github.com/gfx-rs/wgpu/issues/8202
//enable subgroups;

@group(0) @binding(0)
var<storage, read> positions: array<vec3<f32>>;

@group(0) @binding(1)
var<storage, read_write> boundaries: array<u32>;

override WORKGROUP_SIZE: u32;
override CELL_SIZE: f32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = global_invocation_id.x +
        (global_invocation_id.y * WORKGROUP_SIZE * num_workgroups.x) +
        (global_invocation_id.z * WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y);

    if global_index == 0 {
        boundaries[global_index] = 0;
    }
    if global_index >= arrayLength(&positions) {
        return;
    }

    let prev_position = positions[global_index - 1];
    let position = positions[global_index];

    let pref_cell_id = vec3i(floor(prev_position / CELL_SIZE));
    let cell_id = vec3i(floor(position / CELL_SIZE));

    let boundary = pref_cell_id != cell_id;

    if boundary.x || boundary.y || boundary.z {
        boundaries[global_index] = 1;
    } else {
        boundaries[global_index] = 0;
    }
}
