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
var<storage, read> data: array<u32>;

@group(0) @binding(1)
var<storage, read_write> final_data: array<u32>;

var<immediate> max_level: u32;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(subgroup_invocation_id) subgroup_invocation_id: u32,
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = global_invocation_id.x +
        (global_invocation_id.y * WORKGROUP_SIZE * num_workgroups.x) +
        (global_invocation_id.z * WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y);

    let array_length = arrayLength(&data);

    var offset_from_level = 0u;
    if subgroup_invocation_id < max_level {

        var stride = 1u;
        for (var i: u32 = 0; i < subgroup_invocation_id; i++) {
            stride *= subgroup_size;
        }
        let next_stride = stride * subgroup_size;

        let lookup_index = (global_index / stride) * stride;
        let next_lookup_index = (global_index / next_stride) * next_stride;

        if lookup_index > 0 && lookup_index != next_lookup_index {
            offset_from_level = data[lookup_index - 1];
        }
    }

    let subgroup_offset: u32 = subgroupAdd(offset_from_level);

    var my_data = 0u;
    if subgroup_invocation_id > 0 && global_index < array_length {
        my_data = data[global_index - 1];
    }

    my_data = subgroup_offset + my_data;

    if global_index < array_length {
        final_data[global_index] = my_data;
    }
}
