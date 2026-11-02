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

import super::util::{Indirect, find_dispatch_xyz};

@group(0) @binding(0)
var<storage, read_write> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> counts_indirect: Indirect;

@group(0) @binding(2)
var<storage, read_write> status: atomic<u32>;

override DISPATCH_LIMIT: u32;
override DISPATCH_WORKGROUP_SIZE: u32;

override WORKGROUP_SIZE: u32;
override BIT_COUNT: u32;
override SHADER_ID: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(subgroup_size) subgroup_size: u32,
) {
    if any(global_invocation_id != vec3(0)) {
        return;
    }

    if is_error() {
        counts_indirect.x = 0;
        counts_indirect.y = 0;
        counts_indirect.z = 0;
        counts_indirect.len = 0;
        return;
    }
    set_shader_id();

    let counter_count = 1u << BIT_COUNT;
    let subgroups_per_workgroup = DISPATCH_WORKGROUP_SIZE / subgroup_size;
    let actual_workgroup_count = indirect.x * indirect.y * indirect.z;
    let len = actual_workgroup_count * subgroups_per_workgroup * counter_count;

    let dispatch_xyz = find_dispatch_xyz(DISPATCH_LIMIT, DISPATCH_WORKGROUP_SIZE, len);
    counts_indirect.x = dispatch_xyz.x;
    counts_indirect.y = dispatch_xyz.y;
    counts_indirect.z = dispatch_xyz.z;
    counts_indirect.len = len;
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
