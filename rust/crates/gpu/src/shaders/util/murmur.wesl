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

// This is derived from https://github.com/stusmall/murmur3
// and tested against it, its license is in ../LICENSE-murmur3-MIT.txt

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

fn murmur_of_node_id(node_id: vec3i, collider_bits: u32) -> u32 {
    return murmur3_x86_32_3u32(
        i32_to_ordered_u32(node_id.x),
        i32_to_ordered_u32(node_id.y),
        i32_to_ordered_u32(node_id.z),
        collider_bits,
    );
}

fn i32_to_ordered_u32(x: i32) -> u32 {
    return bitcast<u32>(x) ^ 0x80000000u;
}
