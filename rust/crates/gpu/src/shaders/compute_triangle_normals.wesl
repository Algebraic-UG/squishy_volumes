// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{get_global_index, Triangle, try_normalize};

@group(0) @binding(0)
var<storage, read_write> vertex_positions: array<vec3f>;

@group(0) @binding(1)
var<storage, read_write> triangle_indices: array<Triangle>;

@group(0) @binding(2)
var<storage, read_write> triangle_normals: array<vec3f>;

@group(0) @binding(3)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(num_workgroups) num_workgroups: vec3u,
    @builtin(global_invocation_id) global_invocation_id: vec3u,
) {
    if atomicLoad(&status) != 0 {
        return;
    }
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= arrayLength(&triangle_indices) {
        return;
    }

    let triangle = triangle_indices[global_index];

    let a = vertex_positions[triangle.a];
    let b = vertex_positions[triangle.b];
    let c = vertex_positions[triangle.c];

    triangle_normals[global_index] = try_normalize(cross(b - a, c - a), vec3f(0.));
}
