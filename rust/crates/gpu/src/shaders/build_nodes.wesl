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

import super::util::{PositionAndColliderBits, NodeIdAndColliderBits, get_global_index, node_offset, position_to_low_node};

@group(0) @binding(0)
var<storage, read_write> positions_and_collider_bits: array<PositionAndColliderBits>;

@group(0) @binding(1)
var<storage, read_write> owns: array<u32>;

@group(0) @binding(2)
var<storage, read_write> offsets: array<u32>;

@group(0) @binding(3)
var<storage, read_write> node_ids_and_collider_bits: array<NodeIdAndColliderBits>;

@group(0) @binding(4)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;
override GRID_NODE_SIZE: f32;

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
    if global_index >= arrayLength(&positions_and_collider_bits) {
        return;
    }

    let position_and_collider_bits = positions_and_collider_bits[global_index];
    let own = owns[global_index];
    var node_index = offsets[global_index];

    let low_node_id = position_to_low_node(GRID_NODE_SIZE, position_and_collider_bits.position);

    for (var node = 0u; node < 27; node++) {
        if (own & (1u << node)) == 0 {
            continue;
        }

        node_ids_and_collider_bits[node_index].node_id = low_node_id + node_offset(node);
        node_ids_and_collider_bits[node_index].collider_bits = position_and_collider_bits.collider_bits;

        node_index += 1;
    }
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
