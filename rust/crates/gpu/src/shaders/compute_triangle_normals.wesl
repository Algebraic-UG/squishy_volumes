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

import super::util::{get_global_index, Triangle, try_normalize};

@group(0) @binding(0)
var<storage, read_write> vertex_positions: array<vec3f>;

@group(0) @binding(1)
var<storage, read_write> triangle_indices: array<Triangle>;

@group(0) @binding(2)
var<storage, read_write> triangle_normals: array<vec3f>;

@group(0) @binding(3)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(num_workgroups) num_workgroups: vec3u,
    @builtin(global_invocation_id) global_invocation_id: vec3u,
) {
    if is_error() {
        return;
    }
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index == 0 {
        set_shader_id();
    }
    if global_index >= arrayLength(&triangle_indices) {
        return;
    }

    let triangle = triangle_indices[global_index];

    let a = vertex_positions[triangle.a];
    let b = vertex_positions[triangle.b];
    let c = vertex_positions[triangle.c];

    triangle_normals[global_index] = try_normalize(cross(b - a, c - a), vec3f(0.));
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
