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
var<storage, read_write> counts_indirect: Indirect;

override DISPATCH_LIMIT: u32;
override WORKGROUP_SIZE: u32;
override BIT_COUNT: u32;

@compute @workgroup_size(1)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
) {
    let counter_count = 1u << BIT_COUNT;
    let subgroups_per_workgroup = WORKGROUP_SIZE / subgroup_size;
    let actual_workgroup_count = indirect.x * indirect.y * indirect.z;
    let len = actual_workgroup_count * subgroups_per_workgroup * counter_count;

    let dispatch_xyz = find_dispatch_xyz(len);
    counts_indirect.x = dispatch_xyz.x;
    counts_indirect.y = dispatch_xyz.y;
    counts_indirect.z = dispatch_xyz.z;
    counts_indirect.len = len;
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


