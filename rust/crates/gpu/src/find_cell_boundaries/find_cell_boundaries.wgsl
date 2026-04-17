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
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> positions: array<vec3<f32>>;

@group(0) @binding(2)
var<storage, read_write> boundaries: array<u32>;

override WORKGROUP_SIZE: u32;
override CELL_SIZE: f32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(num_workgroups, global_invocation_id);
    if global_index >= indirect.len {
        return;
    }

    if global_index + 1 == indirect.len {
        boundaries[global_index] = 1;
        return;
    }

    let position = positions[global_index];
    let next_position = positions[global_index + 1];

    let cell_id = vec3i(floor(position / CELL_SIZE));
    let next_cell_id = vec3i(floor(next_position / CELL_SIZE));

    let boundary = cell_id != next_cell_id;

    if boundary.x || boundary.y || boundary.z {
        boundaries[global_index] = 1;
    } else {
        boundaries[global_index] = 0;
    }
}

fn get_global_index(num_workgroups: vec3<u32>, global_invocation_id: vec3<u32>) -> u32 {
    return global_invocation_id.x +
        (global_invocation_id.y * WORKGROUP_SIZE * num_workgroups.x) +
        (global_invocation_id.z * WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y);
}

struct Indirect {
    x: u32,
    y: u32,
    z: u32,
    len: u32,
}
