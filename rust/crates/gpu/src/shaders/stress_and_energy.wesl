// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index, first_piola_stress_neo_hookean, first_piola_stress_inviscid, elastic_energy_neo_hookean, elastic_energy_inviscid, ParticleParameters};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> position_gradients: array<mat3x3f>;

@group(0) @binding(2)
var<storage, read_write> particle_parameters: array<ParticleParameters>;

@group(0) @binding(3)
var<storage, read_write> stresses: array<mat3x3f>;

@group(0) @binding(4)
var<storage, read_write> energies: array<f32>;

@group(0) @binding(5)
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
    if global_index >= indirect.len {
        return;
    }

    let position_gradient = position_gradients[global_index];
    let parameters = particle_parameters[global_index];

    if (parameters.flags & 1) > 0 {
        let mu = parameters.a;
        let lambda = parameters.b;

        stresses[global_index] = first_piola_stress_neo_hookean(mu, lambda, position_gradient);
        energies[global_index] = elastic_energy_neo_hookean(mu, lambda, position_gradient);
        return;
    }

    if (parameters.flags & 2) > 0 {
        let exponent = parameters.a;
        let bulk_modulus = parameters.b;

        stresses[global_index] = first_piola_stress_inviscid(bulk_modulus, exponent, position_gradient);
        energies[global_index] = elastic_energy_inviscid(bulk_modulus, exponent, position_gradient);
        return;
    }
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
