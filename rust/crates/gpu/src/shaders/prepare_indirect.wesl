// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index, find_dispatch_xyz, int_pow, div_ceil};

@group(0) @binding(0)
var<storage, read_write> numbers_indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> indirect_levels: array<Indirect>;

@group(0) @binding(2)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override DISPATCH_LIMIT: u32;
override SHADER_ID: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= arrayLength(&indirect_levels) {
        return;
    }

    if is_error() {
        indirect_levels[global_index].len = 0;
        indirect_levels[global_index].x = 0;
        indirect_levels[global_index].y = 0;
        indirect_levels[global_index].z = 0;
        return;
    }
    if global_index == 0 {
        set_shader_id();
    }

    let stride = int_pow(subgroup_size, global_index);
    let len = div_ceil(numbers_indirect.len, stride);
    let dispatch_xyz = find_dispatch_xyz(DISPATCH_LIMIT, WORKGROUP_SIZE, len);

    indirect_levels[global_index].len = len;
    indirect_levels[global_index].x = dispatch_xyz.x;
    indirect_levels[global_index].y = dispatch_xyz.y;
    indirect_levels[global_index].z = dispatch_xyz.z;
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
