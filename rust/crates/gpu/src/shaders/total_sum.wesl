// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index, int_pow, int_log};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> intermediate: array<u32>;

@group(0) @binding(2)
var<storage, read_write> total_sum: u32;

@group(0) @binding(3)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(subgroup_invocation_id) subgroup_invocation_id: u32,
) {
    if is_error() {
        return;
    }
    var offset_from_level = 0u;

    let max_level = int_log(indirect.len * subgroup_size - 1, subgroup_size);

    if subgroup_invocation_id < max_level {
        let stride = int_pow(subgroup_size, subgroup_invocation_id);
        let next_stride = stride * subgroup_size;

        let lookup_index = (indirect.len / stride) * stride;
        let next_lookup_index = (indirect.len / next_stride) * next_stride;

        if lookup_index > 0 && lookup_index != next_lookup_index {
            offset_from_level = intermediate[lookup_index - 1];
        }
    }

    let subgroup_offset: u32 = subgroupAdd(offset_from_level);

    if subgroup_invocation_id != 0 {
        return;
    }

    total_sum = subgroup_offset;
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
