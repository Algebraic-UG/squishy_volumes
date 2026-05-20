// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index, position_to_cell, block_offset};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> cell_ids: array<vec3i>;

@group(0) @binding(2)
var<storage, read_write> cell_owns: array<u32>;

@group(0) @binding(3)
var<storage, read_write> block_offsets: array<u32>;

@group(0) @binding(4)
var<storage, read_write> block_ids: array<vec3i>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= indirect.len {
        return;
    }

    let cell_id = cell_ids[global_index];
    let cell_own = cell_owns[global_index];
    var offset = block_offsets[global_index];

    for (var block = 0u; block < 8; block++) {
        if (cell_own & block) == 0 {
            continue;
        }
        block_ids[offset] = cell_id + block_offset(block);
        offset++;
    }
}
