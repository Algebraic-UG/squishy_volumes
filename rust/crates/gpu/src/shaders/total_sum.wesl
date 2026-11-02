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
    if subgroup_invocation_id == 0 {
        set_shader_id();
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
