// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

//https://github.com/gfx-rs/wgpu/issues/8202
//enable subgroups;

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> cells: array<vec3<i32>>;

@group(0) @binding(2)
var<storage, read_write> hashes: array<u32>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(num_workgroups, global_invocation_id);
    if global_index >= indirect.len {
        return;
    }

    let cell = cells[global_index];
    hashes[global_index] = murmur3_x86_32_3u32(
        i32_to_ordered_u32(cell.x),
        i32_to_ordered_u32(cell.y),
        i32_to_ordered_u32(cell.z),
        0,
    );
}

struct Indirect {
    x: u32,
    y: u32,
    z: u32,
    len: u32,
}

fn get_global_index(num_workgroups: vec3<u32>, global_invocation_id: vec3<u32>) -> u32 {
    return global_invocation_id.x +
        (global_invocation_id.y * WORKGROUP_SIZE * num_workgroups.x) +
        (global_invocation_id.z * WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y);
}

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

fn i32_to_ordered_u32(x: i32) -> u32 {
    return bitcast<u32>(x) ^ 0x80000000u;
}
