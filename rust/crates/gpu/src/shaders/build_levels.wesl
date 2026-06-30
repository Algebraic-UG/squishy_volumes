// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> intermediate: array<u32>;

@group(0) @binding(2)
var<storage, read_write> status: atomic<u32>;

var<immediate> stride: u32;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    if atomicLoad(&status) != 0 {
        return;
    }
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);

    let index = stride - 1 + global_index * stride;

    var my_data = 0u;
    if index < indirect.len {
        my_data = intermediate[index];
    }

    my_data = subgroupInclusiveAdd(my_data);

    if index < indirect.len {
        intermediate[index] = my_data;
    }
}
