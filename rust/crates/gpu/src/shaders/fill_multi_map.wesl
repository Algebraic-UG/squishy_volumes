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

// Inspired by https://nosferalatu.com/SimpleGPUHashTable.html

import super::util::{Indirect, NodeIdAndColliderBits, get_global_index, murmur_of_node_id, TABLE_TRIES_EXCEEDED, TABLE_ENTRY_MISSING};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> node_ids_and_collider_bits: array<NodeIdAndColliderBits>;

@group(0) @binding(2)
var<storage, read_write> hash_table_multi: array<u32>;

@group(0) @binding(3)
var<storage, read_write> multi_counts: array<atomic<u32>>;

@group(0) @binding(4)
var<storage, read_write> multi_offsets: array<u32>;

@group(0) @binding(5)
var<storage, read_write> multi: array<u32>;

@group(0) @binding(6)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;
override TABLE_TRIES: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    if is_error() {
        return;
    }
    var global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index == 0 {
        set_shader_id();
    }
    if global_index >= indirect.len {
        return;
    }

    let node_id = node_ids_and_collider_bits[global_index].node_id;

    // table length must be a power of two
    let table_mask = arrayLength(&hash_table_multi) - 1;
    let hash = murmur_of_node_id(node_id, 0);

    var tries = 0u;
    var slot = hash & table_mask;
    if hash_table_multi[slot] == 0 {
        atomicOr(&status, TABLE_ENTRY_MISSING);
        return;
    }
    var node_index = hash_table_multi[slot] - 1;
    loop {
        if tries > TABLE_TRIES {
            atomicOr(&status, TABLE_TRIES_EXCEEDED);
            return;
        }

        if all(node_id == node_ids_and_collider_bits[node_index].node_id) {
            break;
        }

        slot += 1;
        slot &= table_mask;
        tries += 1;

        if hash_table_multi[slot] == 0 {
            atomicOr(&status, TABLE_ENTRY_MISSING);
            return;
        }

        node_index = hash_table_multi[slot] - 1;
    }

    let local_offset = atomicAdd(&multi_counts[node_index], 1);
    let index = local_offset + multi_offsets[node_index];

    multi[index] = global_index;
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
