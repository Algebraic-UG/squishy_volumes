// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

import super::util::{Indirect, get_global_index};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> keys: array<u32>;

@group(0) @binding(2)
var<storage, read_write> prefixes: array<u32>;

@group(0) @binding(3)
var<storage, read_write> indices_out: array<u32>;

@group(0) @binding(4)
var<storage, read_write> status: atomic<u32>;

var<immediate> bit_offset: u32;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;
override BIT_COUNT: u32;

@compute @workgroup_size(WORKGROUP_SIZE)

fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(num_subgroups) num_subgroups: u32,
    @builtin(subgroup_invocation_id) subgroup_invocation_id: u32,
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    if is_error() {
        return;
    }

    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index == 0 {
        set_shader_id();
    }
    let global_index_valid = global_index < indirect.len;

    let counter_end = 1u << BIT_COUNT;
    let mask = counter_end - 1;

    // these need to be reordered
    var key = 0u;
    if global_index_valid {
        key = keys[global_index];
    }

    // we reorder by this value
    let sub_key = (key >> bit_offset) & mask;

    // this invocation loads the global offset of the respective sub key value
    // which might be different from the sub key above
    let subgroup_index = global_index / subgroup_size;
    let total_num_subgroups = num_subgroups * num_workgroups.x * num_workgroups.y * num_workgroups.z;
    let count_index = total_num_subgroups * subgroup_invocation_id + subgroup_index;
    var global_offset = 0u;
    if subgroup_invocation_id < counter_end {
        global_offset = prefixes[count_index];
    }

    // we then retrieve the global offset needed for the sub key above
    var global_index_out: u32 = subgroupShuffle(global_offset, sub_key);

    // then we need to figure out the local offset
    for (var counter: u32 = 0u; counter < counter_end; counter++) {
        var towards_counter = 0u;
        if global_index_valid && sub_key == counter {
            towards_counter = 1u;
        }

        let local_sub_key_offset: u32 = subgroupExclusiveAdd(towards_counter);
        if sub_key == counter {
            global_index_out += local_sub_key_offset;
        }
    }

    // finish reordering
    if global_index_valid {
        indices_out[global_index_out] = global_index;
    }
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
