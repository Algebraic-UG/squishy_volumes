// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

@group(0) @binding(0)
var<storage, read> prefix_sums: array<u32>;

@group(0) @binding(1)
var<storage, read_write> limits: array<u32>;

@group(0) @binding(2)
var<storage, read_write> indirect: array<u32>;

override DISPATCH_LIMIT: u32;

fn div_ceil(left: u32, right: u32) -> u32 {
    if left == 0 {
        return 0;
    }
    return 1 + ((left - 1) / right);
}

// this is the size for the dispatch later
override WORKGROUP_SIZE: u32;

@compute @workgroup_size(8)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
) {
    let global_index = global_invocation_id.x;
    if global_index >= 8 {
        return;
    }

    var len = 0u;
    if global_index == 0 {
        len = limits[0];
    }
    len = subgroupBroadcast(len, 0u);

    {
        let subgroups_per_workgroup = WORKGROUP_SIZE / subgroup_size;
        let workgroup_count = div_ceil(len, WORKGROUP_SIZE);
        let x = min(DISPATCH_LIMIT, workgroup_count);
        let y = min(DISPATCH_LIMIT, div_ceil(workgroup_count, DISPATCH_LIMIT));
        let z = min(DISPATCH_LIMIT, div_ceil(workgroup_count, DISPATCH_LIMIT * DISPATCH_LIMIT));
        let actual_workgroup_count = x * y * z;
        len = actual_workgroup_count * subgroups_per_workgroup * 8;
    }

    let stride = len / 8;
    let index = stride - 1 + global_index * stride;

    var start = 0u;
    if global_index > 0 {
        let prev_index = stride - 1 + (global_index - 1) * stride;
        start = prefix_sums[prev_index];
    }

    let end = prefix_sums[index];

    limits[global_index] = end;

    let count = end - start;

    let workgroup_count = div_ceil(count, WORKGROUP_SIZE);
    let x = min(DISPATCH_LIMIT, workgroup_count);
    let y = min(DISPATCH_LIMIT, div_ceil(workgroup_count, DISPATCH_LIMIT));
    let z = min(DISPATCH_LIMIT, div_ceil(workgroup_count, DISPATCH_LIMIT * DISPATCH_LIMIT));

    indirect[global_index * 3 + 0] = x;
    indirect[global_index * 3 + 1] = y;
    indirect[global_index * 3 + 2] = z;
}
