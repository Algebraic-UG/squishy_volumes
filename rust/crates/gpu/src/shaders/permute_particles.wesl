// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index, ParticleParameters};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> permutation: array<u32>;

@group(0) @binding(2)
var<storage, read_write> indices_in: array<u32>;

@group(0) @binding(3)
var<storage, read_write> masses_in: array<f32>;

@group(0) @binding(4)
var<storage, read_write> initial_volumes_in: array<f32>;

@group(0) @binding(5)
var<storage, read_write> parameters_in: array<ParticleParameters>;

@group(0) @binding(6)
var<storage, read_write> positions_in: array<vec3<f32>>;

@group(0) @binding(7)
var<storage, read_write> position_gradients_in: array<mat3x3<f32>>;

@group(0) @binding(8)
var<storage, read_write> indices_out: array<u32>;

@group(0) @binding(9)
var<storage, read_write> masses_out: array<f32>;

@group(0) @binding(10)
var<storage, read_write> initial_volumes_out: array<f32>;

@group(0) @binding(11)
var<storage, read_write> parameters_out: array<ParticleParameters>;

@group(0) @binding(12)
var<storage, read_write> positions_out: array<vec3<f32>>;

@group(0) @binding(13)
var<storage, read_write> position_gradients_out: array<mat3x3<f32>>;

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

    let prior_position = permutation[global_index];

    indices_out[global_index] = indices_in[prior_position];
    masses_out[global_index] = masses_in[prior_position];
    initial_volumes_out[global_index] = initial_volumes_in[prior_position];
    parameters_out[global_index] = parameters_in[prior_position];
    positions_out[global_index] = positions_in[prior_position];
    position_gradients_out[global_index] = position_gradients_in[prior_position];
}
