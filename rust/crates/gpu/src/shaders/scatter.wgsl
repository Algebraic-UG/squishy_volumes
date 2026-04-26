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
var<storage, read> indirect_particles: Indirect;

@group(0) @binding(1)
var<storage, read> indirect_colors_batch: array<Indirect>;

@group(0) @binding(2)
var<storage, read_write> indices: array<u32>;

@group(0) @binding(3)
var<storage, read_write> cells: array<vec3i>;

@group(0) @binding(4)
var<storage, read_write> index_ranges: array<u32>;

@group(0) @binding(5)
var<storage, read_write> owns: array<u32>;

@group(0) @binding(6)
var<storage, read_write> block_table: array<u32>;

@group(0) @binding(7)
var<storage, read_write> block_offsets: array<u32>;

@group(0) @binding(8)
var<storage, read_write> positions: array<vec3<f32>>;

@group(0) @binding(9)
var<storage, read_write> blocks: array<array<vec4f,8>>;

var<immediate> color: u32;

override WORKGROUP_SIZE: u32;
override CELL_SIZE: f32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(subgroup_invocation_id) subgroup_invocation_id: u32,
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(num_workgroups, global_invocation_id);
    var color_index = global_index / subgroup_size;
    if color > 0 {
        color_index += indirect_colors_batch[color - 1].len;
    }
    if color_index >= indirect_colors_batch[color].len {
        return;
    }
    let cell_index = indices[color_index];

    let cell_id = cells[cell_index];

    let grid_node_rounds = 64u / subgroup_size;
    let block = grid_node_rounds * subgroup_invocation_id / 8;

    let block_id = cell_id + block_offset(block);
    let hash = murmur_of_cell(block_id);
    let table_mask = arrayLength(&block_table) - 1;
    let index_mask = (1u << 29) - 1;

    var slot = hash & table_mask;

    var owning_block = 0u;
    var owning_cell_index = 0u;
    loop {
        let block_and_index = block_table[slot];
        owning_block = block_and_index >> 29;
        owning_cell_index = (block_and_index & index_mask) - 1;
        let owning_cell_id = cells[owning_cell_index];
        if all(owning_cell_id + block_offset(owning_block) == block_id) {
            break;
        }
        slot += 1;
        slot &= table_mask;
    }

    let block_mask = (1u << owning_block) - 1;
    let blocks_before = countOneBits(owns[owning_cell_index] & block_mask);
    let block_index = block_offsets[owning_cell_index] + blocks_before;

    var particle_start = 0u;
    if cell_index > 0 {
        particle_start = index_ranges[cell_index - 1];
    }
    let particle_end = index_ranges[cell_index];
    let particle_count = particle_end - particle_start;
    let particle_rounds = div_ceil(particle_count, subgroup_size);

    let node_start = subgroup_invocation_id % (8 / grid_node_rounds);
    let node_stride = 8 / grid_node_rounds;

    let node_id_start = block_id * 2 - vec3i(1);

    for (var particle_round = 0u; particle_round < particle_rounds; particle_round++) {
        let particle_index = particle_start + particle_round * subgroup_size + subgroup_invocation_id;
        let particle_valid = particle_index < particle_end;
        var normalized_position = vec3f(0);
        if particle_valid {
            normalized_position = positions[particle_index] / (CELL_SIZE * 0.5);
        }

        for (var grid_node_round = 0u; grid_node_round < grid_node_rounds; grid_node_round++) {
            let node_index = node_start + node_stride * grid_node_round;
            var node = blocks[block_index][node_index];
            var node_id = node_id_start + block_offset(node_index);

            for (var write_round = 0u; write_round < subgroup_size; write_round++) {
                node = subgroupShuffle(node, (subgroup_invocation_id + 1) % subgroup_size);
                node_id = subgroupShuffle(node_id, (subgroup_invocation_id + 1) % subgroup_size);

                if !particle_valid {
                    continue;
                }

                let to_grid_node = vec3f(node_id) - normalized_position;

                let weight = kernel_quadratic(to_grid_node.x) * kernel_quadratic(to_grid_node.y) * kernel_quadratic(to_grid_node.z);

                node.w += weight;
            }

            blocks[block_index][node_index] = node;
        }
    }
}

fn get_global_index(num_workgroups: vec3<u32>, global_invocation_id: vec3<u32>) -> u32 {
    return global_invocation_id.x +
        (global_invocation_id.y * WORKGROUP_SIZE * num_workgroups.x) +
        (global_invocation_id.z * WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y);
}

fn div_ceil(left: u32, right: u32) -> u32 {
    if left == 0 {
        return 0;
    }
    return 1 + ((left - 1) / right);
}

struct Indirect {
    x: u32,
    y: u32,
    z: u32,
    len: u32,
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

fn kernel_quadratic(signed_x: f32) -> f32 {
    let x = abs(signed_x);
    if x < 1. / 2. {
        return 3. / 4. - x * x;
    } else if x < 3. / 2. {
        return 1. / 2. * (3. / 2. - x) * (3. / 2. - x);
    } else {
        return 0.;
    }
}
