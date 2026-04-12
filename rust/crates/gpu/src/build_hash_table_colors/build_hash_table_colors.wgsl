// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

//https://github.com/gfx-rs/wgpu/issues/8202
//enable subgroups;

// Inspired by https://nosferalatu.com/SimpleGPUHashTable.html

@group(0) @binding(0)
var<storage, read> cells: array<vec3<i32>>;

@group(0) @binding(1)
var<storage, read> limits: array<u32>;

@group(0) @binding(2)
var<storage, read_write> slots: array<atomic<u32>>;

@group(0) @binding(3)
var<storage, read_write> owns: array<u32>;

var<immediate> color: u32;

override WORKGROUP_SIZE: u32;

// This is derived from https://github.com/stusmall/murmur3
// and tested against it

fn rotl32(x: u32, r: u32) -> u32 {
    return (x << r) | (x >> (32u - r));
}

fn fmix32(h_in: u32) -> u32 {
    var h = h_in;
    h = h ^ (h >> 16u);
    h = h * 0x85ebca6bu;
    h = h ^ (h >> 13u);
    h = h * 0xc2b2ae35u;
    h = h ^ (h >> 16u);
    return h;
}

fn murmur3_mix_block(h1_in: u32, k1_in: u32) -> u32 {
    let c1: u32 = 0xcc9e2d51u;
    let c2: u32 = 0x1b873593u;

    var h1 = h1_in;
    var k1 = k1_in;

    k1 = k1 * c1;
    k1 = rotl32(k1, 15u);
    k1 = k1 * c2;

    h1 = h1 ^ k1;
    h1 = rotl32(h1, 13u);
    h1 = h1 * 5u + 0xe6546b64u;

    return h1;
}

fn murmur3_x86_32_3u32(a: u32, b: u32, c: u32, seed: u32) -> u32 {
    var h1 = seed;

    h1 = murmur3_mix_block(h1, a);
    h1 = murmur3_mix_block(h1, b);
    h1 = murmur3_mix_block(h1, c);

    h1 = h1 ^ 12u;

    return fmix32(h1);
}

fn murmur_of_cell(cell: vec3i) -> u32 {
    return murmur3_x86_32_3u32(
        i32_to_ordered_u32(cell.x),
        i32_to_ordered_u32(cell.y),
        i32_to_ordered_u32(cell.z),
        0,
    );
}

fn i32_to_ordered_u32(x: i32) -> u32 {
    return bitcast<u32>(x) ^ 0x80000000u;
}

fn block_offset(block: u32) -> vec3i {
    var offset = vec3i(0);
    if (block & 1) == 1 {
        offset.x = 1;
    }
    if (block & 2) == 2 {
        offset.y = 1;
    }
    if (block & 4) == 4 {
        offset.z = 1;
    }
    return offset;
}

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    var global_index = global_invocation_id.x +
        (global_invocation_id.y * WORKGROUP_SIZE * num_workgroups.x) +
        (global_invocation_id.z * WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y);
    if color > 0 {
        global_index += limits[color - 1];
    }

    if global_index >= limits[color] {
        return;
    }
    let cell = cells[global_index];

    // table length must be a power of two
    let table_mask = arrayLength(&slots) - 1;
    let index_mask = (1u << 29) - 1;
    var own = 0u;
    for (var block = 0u; block < 8; block++) {
        let offset_cell = cell + block_offset(block);
        let block_and_index = (block << 29) | (global_index + 1);

        let hash = murmur_of_cell(offset_cell);

        var slot = hash & table_mask;
        loop {
            let result = atomicCompareExchangeWeak(&slots[slot], 0, block_and_index);

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

