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

import super::util::{Indirect, find_dispatch_xyz, INDIRECT_LIMIT_EXCEEDED};

@group(0) @binding(0)
var<storage, read> len: u32;

@group(0) @binding(1)
var<storage, read_write> new_indirect: Indirect;

@group(0) @binding(2)
var<storage, read_write> status: atomic<u32>;

var<immediate> limit: u32;

override DISPATCH_LIMIT: u32;
override DISPATCH_WORKGROUP_SIZE: u32;

override WORKGROUP_SIZE: u32;
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
        new_indirect.x = 0;
        new_indirect.y = 0;
        new_indirect.z = 0;
        return;
    }
    set_shader_id();

    if len > limit {
        atomicOr(&status, INDIRECT_LIMIT_EXCEEDED);
        new_indirect.x = 0;
        new_indirect.y = 0;
        new_indirect.z = 0;
        new_indirect.len = len;
        return;
    }

    let dispatch_xzy = find_dispatch_xyz(DISPATCH_LIMIT, DISPATCH_WORKGROUP_SIZE, len);
    new_indirect.x = dispatch_xzy.x;
    new_indirect.y = dispatch_xzy.y;
    new_indirect.z = dispatch_xzy.z;
    new_indirect.len = len;
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
