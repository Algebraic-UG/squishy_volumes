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
var<storage, read_write> permutation: array<u32>;

@group(0) @binding(2)
var<storage, read_write> indices_in: array<u32>;

@group(0) @binding(3)
var<storage, read_write> positions_in: array<vec3f>;

@group(0) @binding(4)
var<storage, read_write> indices_out: array<u32>;

@group(0) @binding(5)
var<storage, read_write> positions_out: array<vec3f>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)

fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(num_workgroups, global_invocation_id);

    if global_index >= indirect.len {
        return;
    }

    let prior_position = permutation[global_index];

    indices_out[global_index] = indices_in[prior_position];
    positions_out[global_index] = positions_in[prior_position];
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
