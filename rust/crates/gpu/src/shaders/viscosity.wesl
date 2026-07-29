// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index, ParticleParameters, USE_VISCOSITY, trace, from_diagonal_element};

@group(0) @binding(0)
var<storage, read_write> particle_flags: array<u32>;

@group(0) @binding(1)
var<storage, read_write> particle_parameters: array<ParticleParameters>;

@group(0) @binding(2)
var<storage, read_write> particle_velocity_gradients: array<mat3x3f>;

@group(0) @binding(3)
var<storage, read_write> particle_stresses: array<mat3x3f>;

@group(0) @binding(4)
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
    if global_index >= arrayLength(&particle_flags) {
        return;
    }

    let flags = particle_flags[global_index];
    let parameters = particle_parameters[global_index];
    let velocity_gradient = particle_velocity_gradients[global_index];

    if (flags & USE_VISCOSITY) == 0 {
        particle_stresses[global_index] = mat3x3f();
        return;
    }

    particle_stresses[global_index] = cauchy_stress_general_viscosity(
        parameters.viscosity_dynamic,
        parameters.viscosity_bulk,
        velocity_gradient,
    );
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}

fn cauchy_stress_general_viscosity(
    dynamic_viscosity: f32,
    bulk_viscosity: f32,
    velocity_gradient: mat3x3f,
) -> mat3x3f {
    let dynamic_part: mat3x3f = dynamic_viscosity * (velocity_gradient + transpose(velocity_gradient));
    let bulk_part: mat3x3f = bulk_viscosity * from_diagonal_element(trace(velocity_gradient));

    return dynamic_part + bulk_part;
}

