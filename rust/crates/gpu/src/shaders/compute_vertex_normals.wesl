// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{get_global_index, try_normalize};

@group(0) @binding(0)
var<storage, read_write> vertex_triangle_offsets: array<u32>;

@group(0) @binding(1)
var<storage, read_write> vertex_triangle_lists: array<u32>;

@group(0) @binding(2)
var<storage, read_write> vertex_normals: array<vec3f>;

@group(0) @binding(3)
var<storage, read_write> triangle_normals: array<vec3f>;

@group(0) @binding(4)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(num_workgroups) num_workgroups: vec3u,
    @builtin(global_invocation_id) global_invocation_id: vec3u,
) {
    if is_error() {
        return;
    }
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index == 0 {
        set_shader_id();
    }
    if global_index >= arrayLength(&vertex_triangle_offsets) {
        return;
    }

    let triangle_start = vertex_triangle_offsets[global_index];
    let triangle_end = vertex_triangle_offsets[global_index + 1];

    var normal = vec3f(0.);
    for (var lookup = triangle_start; lookup < triangle_end; lookup++) {
        normal += triangle_normals[vertex_triangle_lists[lookup]];
    }

    vertex_normals[global_index] = try_normalize(normal, vec3f(0.));
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
