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

import super::util::{get_global_index, Indirect, int_pow, div_ceil, find_dispatch_xyz, TIME_STEP_ZERO, TimeStepLimits};

@group(0) @binding(0)
var<storage, read_write> limits: array<TimeStepLimits>;

@group(0) @binding(1)
var<storage, read_write> time_step: f32;

@group(0) @binding(2)
var<storage, read_write> status: atomic<u32>;

var<immediate> current_step: u32;

override MAX_TIME_STEP: f32;
override TIME_STEP_HISTORY_LENGTH: u32;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
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

    let valid_entries = current_step + 1;
    let size = min(valid_entries, TIME_STEP_HISTORY_LENGTH);
    let offset = valid_entries - size;

    if global_index >= size {
        return;
    }

    let my_limits = limits[offset + global_index];

    let my_limit = min(
        min(my_limits.time_step_by_velocity, my_limits.time_step_by_deformation),
        min(my_limits.time_step_by_isolated, my_limits.time_step_by_sound),
    );
    let limit = subgroupMin(my_limit);

    if global_index != 0 {
        return;
    }

    time_step = min(MAX_TIME_STEP, limit);

    if time_step == 0. {
        atomicOr(&status, TIME_STEP_ZERO);
    }
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
