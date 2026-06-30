// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, find_dispatch_xyz};

@group(0) @binding(0)
var<storage, read_write> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> counts_indirect: Indirect;

@group(0) @binding(2)
var<storage, read_write> status: atomic<u32>;

override DISPATCH_LIMIT: u32;
override WORKGROUP_SIZE: u32;
override BIT_COUNT: u32;

@compute @workgroup_size(1)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
) {
    if atomicLoad(&status) != 0 {
        return;
    }
    let counter_count = 1u << BIT_COUNT;
    let subgroups_per_workgroup = WORKGROUP_SIZE / subgroup_size;
    let actual_workgroup_count = indirect.x * indirect.y * indirect.z;
    let len = actual_workgroup_count * subgroups_per_workgroup * counter_count;

    let dispatch_xyz = find_dispatch_xyz(DISPATCH_LIMIT, WORKGROUP_SIZE, len);
    counts_indirect.x = dispatch_xyz.x;
    counts_indirect.y = dispatch_xyz.y;
    counts_indirect.z = dispatch_xyz.z;
    counts_indirect.len = len;
}
