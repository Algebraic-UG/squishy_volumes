// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{get_global_index};

@group(0) @binding(0)
var<storage, read_write> vertex_positions_start: array<vec3f>;

@group(0) @binding(1)
var<storage, read_write> vertex_positions_end: array<vec3f>;

@group(0) @binding(2)
var<storage, read_write> vertex_positions: array<vec3f>;

override WORKGROUP_SIZE: u32;

var<immediate> factor: f32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(num_workgroups) num_workgroups: vec3u,
    @builtin(global_invocation_id) global_invocation_id: vec3u,
) {
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= arrayLength(&vertex_positions) {
        return;
    }

    vertex_positions[global_index] = //
        (1. - factor) * vertex_positions_start[global_index] 
        + factor * vertex_positions_end[global_index];
}
