// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index, ParticleParameters, IS_FLUID};
import super::external::wgebra::svd3::{Svd, svd, recompose};

@group(0) @binding(0)
var<storage, read_write> particle_flags: array<u32>;

@group(0) @binding(1)
var<storage, read_write> particle_parameters: array<ParticleParameters>;

@group(0) @binding(2)
var<storage, read_write> particle_position_gradients: array<mat3x3f>;

@group(0) @binding(3)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    if is_error() {
        return;
    }
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index == 0 {
        set_shader_id();
    }
    if global_index >= arrayLength(&particle_parameters) {
        return;
    }

    let flags = particle_flags[global_index];
    let parameters = particle_parameters[global_index];

    if (flags & IS_FLUID) == 0 {
        return;
    }

    particle_position_gradients[global_index] = fluid_plasticity(
        particle_position_gradients[global_index]
    );
}

fn fluid_plasticity(position_gradient: mat3x3f) -> mat3x3f {
    var svd = svd(position_gradient);
    let product = svd.S.x * svd.S.y * svd.S.z;
    svd.S = vec3f(pow(product, 1. / 3.));
    return recompose(svd);
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
