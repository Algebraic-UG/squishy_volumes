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
var<storage, read_write> keys: array<u32>;

var<immediate> dimension: u32;

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

    if global_index >= arrayLength(&positions) {
        return;
    }

    let position = positions[global_index];
    let cell_id_in_dimension = i32(floor(position[dimension] / CELL_SIZE));
    let key = bitcast<u32>(cell_id_in_dimension) ^ 0x80000000u;
    keys[global_index] = key;
}
