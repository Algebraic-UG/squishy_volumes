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

import super::util::{get_global_index, PositionAndColliderBits, TOMBSTONED};

@group(0) @binding(0)
var<storage, read_write> particle_flags: array<u32>;

@group(0) @binding(1)
var<storage, read_write> particle_positions_and_collider_bits: array<PositionAndColliderBits>;

@group(0) @binding(2)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;

override DOMAIN_MIN_X: f32;
override DOMAIN_MIN_Y: f32;
override DOMAIN_MIN_Z: f32;
override DOMAIN_MAX_X: f32;
override DOMAIN_MAX_Y: f32;
override DOMAIN_MAX_Z: f32;

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
    if global_index >= arrayLength(&particle_flags) {
        return;
    }

    var flags = particle_flags[global_index];
    if (flags & TOMBSTONED) != 0 {
        return;
    }

    let position = particle_positions_and_collider_bits[global_index].position;
    let domain_min = vec3f(DOMAIN_MIN_X, DOMAIN_MIN_Y, DOMAIN_MIN_Z);
    let domain_max = vec3f(DOMAIN_MAX_X, DOMAIN_MAX_Y, DOMAIN_MAX_Z);
    if any(position < domain_min) || any(position > domain_max) {
        flags |= TOMBSTONED;
        particle_flags[global_index] = flags;
    }
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
