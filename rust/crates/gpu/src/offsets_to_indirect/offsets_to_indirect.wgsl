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
var<storage, read> prefix_sums: array<u32>;

@group(0) @binding(1)
var<storage, read_write> limits: array<u32>;

@group(0) @binding(2)
var<storage, read_write> indirect: array<u32>;

override DISPATCH_LIMIT: u32;
// this is the size for the dispatch later
override WORKGROUP_SIZE: u32;

fn div_ceil(left: u32, right: u32) -> u32 {
    if left == 0 {
        return 0;
    }
    return 1 + ((left - 1) / right);
}

@compute @workgroup_size(1)
fn main() {
    // size cannot be zero.
    let last = arrayLength(&prefix_sums) - 1;

    let count = prefix_sums[last] + 1;
    limits[0] = count;

    let workgroup_count = div_ceil(count, WORKGROUP_SIZE);
    let x = min(DISPATCH_LIMIT, workgroup_count);
    let y = min(DISPATCH_LIMIT, div_ceil(workgroup_count, DISPATCH_LIMIT));
    let z = min(DISPATCH_LIMIT, div_ceil(workgroup_count, DISPATCH_LIMIT * DISPATCH_LIMIT));

    indirect[0] = x;
    indirect[1] = y;
    indirect[2] = z;
}
