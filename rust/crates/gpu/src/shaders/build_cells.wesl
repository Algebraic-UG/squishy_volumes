// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index, position_to_cell};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> positions: array<vec3f>;

@group(0) @binding(2)
var<storage, read_write> prefixed_boundaries: array<u32>;

@group(0) @binding(3)
var<storage, read_write> cells: array<vec3i>;

@group(0) @binding(4)
var<storage, read_write> index_ranges: array<u32>;

override WORKGROUP_SIZE: u32;
override CELL_SIZE: f32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= indirect.len {
        return;
    }

    let cell_index = prefixed_boundaries[global_index];

    if global_index + 1 != indirect.len {
        if cell_index == prefixed_boundaries[global_index + 1] {
            return;
        }
    }

    cells[cell_index] = position_to_cell(CELL_SIZE, positions[global_index]);
    index_ranges[cell_index] = global_index + 1;
}
