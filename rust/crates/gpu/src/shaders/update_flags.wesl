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

import super::util::{get_global_index, HAS_GOAL, REACHED_FRAME_TIME};

@group(0) @binding(0)
var<storage, read_write> new_flags: array<u32>;

@group(0) @binding(1)
var<storage, read_write> flags: array<u32>;

@group(0) @binding(2)
var<storage, read_write> status: atomic<u32>;

var<immediate> offset: u32;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let error = atomicLoad(&status) & 0x0000FFFFu;
    if error != REACHED_FRAME_TIME {
        return;
    }

    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index == 0 {
        set_shader_id();
    }
    if global_index >= arrayLength(&new_flags) {
        return;
    }

    var f = flags[offset + global_index];
    var new_f = new_flags[global_index];

    f &= ~HAS_GOAL;
    f |= new_f & HAS_GOAL;

    flags[offset + global_index] = f;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
