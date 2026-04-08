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
var<storage, read> permutation: array<u32>;

@group(0) @binding(1)
var<storage, read> positions_in: array<vec3f>;

@group(0) @binding(2)
var<storage, read_write> positions_out: array<vec3f>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)

fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    // The global_index isn't supported yet.
    let global_stride = vec3(
        1,
        WORKGROUP_SIZE * num_workgroups.x,
        WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y,
    );
    let global_index = dot(global_invocation_id, global_stride);

    if global_index >= arrayLength(&permutation) {
        return;
    }

    positions_out[global_index] = positions_in[permutation[global_index]];
}
