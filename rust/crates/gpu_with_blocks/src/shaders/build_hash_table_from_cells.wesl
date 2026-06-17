// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

// Inspired by https://nosferalatu.com/SimpleGPUHashTable.html

import super::util::{Indirect, get_global_index, block_offset, murmur_of_cell};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> cells: array<vec3<i32>>;

@group(0) @binding(2)
var<storage, read_write> block_table: array<atomic<u32>>;

@group(0) @binding(3)
var<storage, read_write> owns: array<u32>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    var global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= indirect.len {
        return;
    }

    let cell = cells[global_index];

    // table length must be a power of two
    let table_mask = arrayLength(&block_table) - 1;
    let index_mask = (1u << 29) - 1;
    var own = 0u;
    for (var block = 0u; block < 8; block++) {
        let offset_cell = cell + block_offset(block);
        let block_and_index = (block << 29) | (global_index + 1);

        let hash = murmur_of_cell(offset_cell);

        var slot = hash & table_mask;
        loop {
            let result = atomicCompareExchangeWeak(&block_table[slot], 0, block_and_index);

            if result.exchanged {
                own |= 1u << block;
                break;
            }

            let old_block: u32 = result.old_value >> 29;
            if block == old_block {
                continue;
            }

            let old_index = (result.old_value & index_mask) - 1;
            let old_cell = cells[old_index];
            if all(old_cell + block_offset(old_block) == offset_cell) {
                break;
            }

            continuing {
                slot += 1;
                slot &= table_mask;
            }
        }
    }

    owns[global_index] = own;
}
