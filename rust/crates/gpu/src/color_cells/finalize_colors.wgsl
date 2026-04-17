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
var<storage, read> limits: array<u32>;

@group(0) @binding(1)
var<storage, read> prefixes: array<u32>;

@group(0) @binding(2)
var<storage, read> cells_in: array<vec3i>;

@group(0) @binding(3)
var<storage, read_write> cells_out: array<vec3i>;

override WORKGROUP_SIZE: u32;

const BIT_COUNT: u32 = 3;

fn i32_to_ordered_u32(x: i32) -> u32 {
    return bitcast<u32>(x) ^ 0x80000000u;
}

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(num_subgroups) num_subgroups: u32,
    @builtin(subgroup_invocation_id) subgroup_invocation_id: u32,
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    // The global_index isn't supported yet.
    let global_stride = vec3(
        1,
        WORKGROUP_SIZE * num_workgroups.x,
        WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y,
    );
    let global_index = dot(global_invocation_id, global_stride);
    let global_index_valid = global_index < limits[0];

    let counter_end = 1u << BIT_COUNT;

    // these need to be reordered
    var cell = vec3i(0);
    if global_index_valid {
        cell = cells_in[global_index];
    }

    // we reorder by this value
    var key = 0u;
    key |= (i32_to_ordered_u32(cell.z) & 1) << 0;
    key |= (i32_to_ordered_u32(cell.y) & 1) << 1;
    key |= (i32_to_ordered_u32(cell.x) & 1) << 2;

    // this invocation loads the global offset of the respective key value
    // which might be different from the key above
    let subgroup_index = global_index / subgroup_size;
    let total_num_subgroups = num_subgroups * num_workgroups.x * num_workgroups.y * num_workgroups.z;
    let count_index = total_num_subgroups * subgroup_invocation_id + subgroup_index;
    var global_offset = 0u;
    if subgroup_invocation_id < counter_end {
        global_offset = prefixes[count_index];
    }

    var global_index_out: u32 = subgroupShuffle(global_offset, key);

    // then we need to figure out the local offset
    for (var counter: u32 = 0u; counter < counter_end; counter++) {
        var towards_counter = 0u;
        if global_index_valid && key == counter {
            towards_counter = 1u;
        }

        let local_key_offset: u32 = subgroupExclusiveAdd(towards_counter);
        if key == counter {
            global_index_out += local_key_offset;
        }
    }

    // finish reordering
    if global_index_valid {
        cells_out[global_index_out] = cell;
    }
}
