// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index, div_ceil, murmur_of_cell, block_offset, kernel_quadratic, first_piola_stress_neo_hookean, first_piola_stress_inviscid, ParticleParameters};

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

// particle stuff

@group(0) @binding(8)
var<storage, read_write> masses: array<f32>;

@group(0) @binding(9)
var<storage, read_write> initial_volumes: array<f32>;

@group(0) @binding(10)
var<storage, read_write> parameters: array<ParticleParameters>;

@group(0) @binding(11)
var<storage, read_write> positions: array<vec3<f32>>;

@group(0) @binding(12)
var<storage, read_write> position_gradients: array<mat3x3<f32>>;

@group(0) @binding(13)
var<storage, read_write> velocities: array<vec3<f32>>;

@group(0) @binding(14)
var<storage, read_write> velocity_gradients: array<mat3x3<f32>>;

// grid stuff

@group(0) @binding(15)
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
    let time_step = 0.001;
    let grid_node_size = CELL_SIZE * 0.5;
    let scaling = time_step * 4. / (grid_node_size * grid_node_size);

    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
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

        var mass = 0.;
        var normalized_position = vec3f(0.);
        var velocity = vec3f(0.);
        var tmp = mat3x3f();
        if particle_valid {
            mass = masses[particle_index];
            normalized_position = positions[particle_index] / grid_node_size;
            velocity = velocities[particle_index];

            let initial_volume = initial_volumes[particle_index];
            let parameter = parameters[particle_index];
            let position_gradient = position_gradients[particle_index];
            let velocity_gradient = velocity_gradients[particle_index];

            var stress = mat3x3f();
            if (parameter.flags & 1) > 0 {
                let mu = parameter.a;
                let lambda = parameter.b;
                stress = first_piola_stress_neo_hookean(mu, lambda, position_gradient);
            }
            if (parameter.flags & 2) > 0 {
                let exponent = parameter.a;
                let bulk_modulus = parameter.b;
                stress = first_piola_stress_inviscid(bulk_modulus, exponent, position_gradient);
            }

            tmp = velocity_gradient * mass - stress * transpose(position_gradient) * scaling * initial_volume;
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

                node += vec4f(velocity * mass + tmp * to_grid_node * grid_node_size, mass) * weight;
            }

            blocks[block_index][node_index] = node;
        }
    }
}
