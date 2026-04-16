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
var<storage, read> numbers_indirect: Indirect;

@group(0) @binding(1)
var<storage, read> numbers: array<u32>;

@group(0) @binding(2)
var<storage, read_write> indirect_levels: array<Indirect>;

override WORKGROUP_SIZE: u32;
override DISPATCH_LIMIT: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(num_workgroups, global_invocation_id);
    if global_index >= arrayLength(&indirect_levels) {
        return;
    }

    let stride = int_pow(subgroup_size, global_index);
    let len = div_ceil(numbers_indirect.len, stride);
    let dispatch_xyz = find_dispatch_xyz(len);

    indirect_levels[global_index].len = len;
    indirect_levels[global_index].x = dispatch_xyz.x;
    indirect_levels[global_index].y = dispatch_xyz.y;
    indirect_levels[global_index].z = dispatch_xyz.z;
}

fn get_global_index(num_workgroups: vec3<u32>, global_invocation_id: vec3<u32>) -> u32 {
    return global_invocation_id.x +
        (global_invocation_id.y * WORKGROUP_SIZE * num_workgroups.x) +
        (global_invocation_id.z * WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y);
}

fn div_ceil(left: u32, right: u32) -> u32 {
    if left == 0 {
        return 0;
    }
    return 1 + ((left - 1) / right);
}

fn int_pow(base: u32, exponent: u32) -> u32 {
    var result = 1u;
    for (var i = 0u; i < exponent; i++) {
        result *= base;
    }
    return result;
}

fn find_dispatch_xyz(len: u32) -> vec3u {
    let workgroup_count = div_ceil(len, WORKGROUP_SIZE);
    let x = min(DISPATCH_LIMIT, workgroup_count);
    let y = min(DISPATCH_LIMIT, div_ceil(workgroup_count, DISPATCH_LIMIT));
    let z = min(DISPATCH_LIMIT, div_ceil(workgroup_count, DISPATCH_LIMIT * DISPATCH_LIMIT));
    return vec3u(x, y, z);
}

struct Indirect {
    x: u32,
    y: u32,
    z: u32,
    len: u32,
}
