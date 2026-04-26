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
var<storage, read> prefix_sums: array<u32>;

@group(0) @binding(2)
var<storage, read_write> indirect_colors: array<Indirect>;

@group(0) @binding(3)
var<storage, read_write> indirect_colors_batch: array<Indirect>;

override DISPATCH_LIMIT: u32;
override WORKGROUP_SIZE: u32;

const COLORS: u32 = 8;

@compute @workgroup_size(COLORS)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
) {
    let global_index = global_invocation_id.x;
    if global_index >= COLORS {
        return;
    }

    var count_len = 0u;
    {
        let subgroups_per_workgroup = WORKGROUP_SIZE / subgroup_size;
        let dispatch_xyz = find_dispatch_xyz(DISPATCH_LIMIT, WORKGROUP_SIZE, indirect.len);
        let actual_workgroup_count = dispatch_xyz.x * dispatch_xyz.y * dispatch_xyz.z;
        count_len = actual_workgroup_count * subgroups_per_workgroup * COLORS;
    }

    let stride = count_len / COLORS;
    let start = prefix_sums[global_index * stride];

    var end = indirect.len;
    if global_index + 1 < COLORS {
        end = prefix_sums[(global_index + 1) * stride];
    }

    indirect_colors[global_index].len = end;
    indirect_colors_batch[global_index].len = end;

    let len = end - start;

    {
        let dispatch_xyz = find_dispatch_xyz(DISPATCH_LIMIT, WORKGROUP_SIZE, len);
        indirect_colors[global_index].x = dispatch_xyz.x;
        indirect_colors[global_index].y = dispatch_xyz.y;
        indirect_colors[global_index].z = dispatch_xyz.z;
    }

    let dispatch_xyz = find_dispatch_xyz(DISPATCH_LIMIT, WORKGROUP_SIZE, len * subgroup_size);
    indirect_colors_batch[global_index].x = dispatch_xyz.x;
    indirect_colors_batch[global_index].y = dispatch_xyz.y;
    indirect_colors_batch[global_index].z = dispatch_xyz.z;
}
