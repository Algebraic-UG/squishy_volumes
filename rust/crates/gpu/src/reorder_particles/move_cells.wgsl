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
var<storage, read> limits: array<u32>;

@group(0) @binding(1)
var<storage, read> indices: array<u32>;

@group(0) @binding(2)
var<storage, read> index_ranges: array<u32>;

@group(0) @binding(3)
var<storage, read> offsets: array<u32>;

@group(0) @binding(4)
var<storage, read> positions_in: array<vec3f>;

@group(0) @binding(5)
var<storage, read_write> positions_out: array<vec3f>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = global_invocation_id.x +
        (global_invocation_id.y * WORKGROUP_SIZE * num_workgroups.x) +
        (global_invocation_id.z * WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y);

    if global_index >= limits[0] {
        return;
    }

    var start = 0u;
    if global_index > 0 {
        start = index_ranges[global_index - 1];
    }
    let end = index_ranges[global_index];

    let offset = offsets[indices[global_index]];
    for (var i: u32 = start; i < end; i++) {
        positions_out[offset + i - start] = positions_in[i];
    }
}
