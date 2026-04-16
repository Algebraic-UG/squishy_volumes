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
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read> intermediate: array<u32>;

@group(0) @binding(2)
var<storage, read_write> prefix_sums: array<u32>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(subgroup_invocation_id) subgroup_invocation_id: u32,
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(num_workgroups, global_invocation_id);

    var offset_from_level = 0u;

    let max_level = int_log(indirect.len * subgroup_size - 1, subgroup_size);

    if subgroup_invocation_id < max_level {
        let stride = int_pow(subgroup_size, subgroup_invocation_id);
        let next_stride = stride * subgroup_size;

        let lookup_index = (global_index / stride) * stride;
        let next_lookup_index = (global_index / next_stride) * next_stride;

        if lookup_index > 0 && lookup_index != next_lookup_index {
            offset_from_level = intermediate[lookup_index - 1];
        }
    }

    let subgroup_offset: u32 = subgroupAdd(offset_from_level);

    var my_data = 0u;
    if subgroup_invocation_id > 0 && global_index < indirect.len {
        my_data = intermediate[global_index - 1];
    }

    my_data = subgroup_offset + my_data;

    if global_index < indirect.len {
        prefix_sums[global_index] = my_data;
    }
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

fn int_pow(base: u32, exponent: u32) -> u32 {
    var result = 1u;
    for (var i = 0u; i < exponent; i++) {
        result *= base;
    }
    return result;
}

fn int_log(value: u32, base: u32) -> u32 {
    if base < 2 {
        return 0;
    }
    var result = 1u;
    var power = 1u;
    while power < value {
        result += 1;
        power *= base;
    }
    return result;
}
