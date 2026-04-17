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
var<storage, read_write> indirect: Indirect;

@group(0) @binding(1)
var<storage, read> offsets: array<u32>;

@group(0) @binding(2)
var<storage, read_write> new_indirect: Indirect;

override DISPATCH_LIMIT: u32;
override WORKGROUP_SIZE: u32;

@compute @workgroup_size(1)
fn main() {
    // this better not be zero.
    let last = indirect.len - 1;

    let len = offsets[last] + 1;

    let dispatch_xzy = find_dispatch_xyz(len);
    new_indirect.x = dispatch_xzy.x;
    new_indirect.y = dispatch_xzy.y;
    new_indirect.z = dispatch_xzy.z;
    new_indirect.len = len;
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

fn div_ceil(left: u32, right: u32) -> u32 {
    if left == 0 {
        return 0;
    }
    return 1 + ((left - 1) / right);
}

fn find_dispatch_xyz(len: u32) -> vec3u {
    let workgroup_count = div_ceil(len, WORKGROUP_SIZE);
    let x = min(DISPATCH_LIMIT, workgroup_count);
    let y = min(DISPATCH_LIMIT, div_ceil(workgroup_count, DISPATCH_LIMIT));
    let z = min(DISPATCH_LIMIT, div_ceil(workgroup_count, DISPATCH_LIMIT * DISPATCH_LIMIT));
    return vec3u(x, y, z);
}
