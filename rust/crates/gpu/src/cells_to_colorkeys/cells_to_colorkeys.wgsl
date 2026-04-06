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
var<storage, read> cells: array<vec3<i32>>;

@group(0) @binding(1)
var<storage, read_write> keys: array<u32>;

override WORKGROUP_SIZE: u32;

fn i32_to_ordered_u32(x: i32) -> u32 {
    return bitcast<u32>(x) ^ 0x80000000u;
}

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = global_invocation_id.x +
        (global_invocation_id.y * WORKGROUP_SIZE * num_workgroups.x) +
        (global_invocation_id.z * WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y);

    if global_index >= arrayLength(&cells) {
        return;
    }

    let cell = cells[global_index];

    var key = 0u;
    key |= (i32_to_ordered_u32(cell.z) & 1) << 0;
    key |= (i32_to_ordered_u32(cell.y) & 1) << 1;
    key |= (i32_to_ordered_u32(cell.x) & 1) << 2;

    keys[global_index] = key;
}
