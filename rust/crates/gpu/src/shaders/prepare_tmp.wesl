// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{PositionAndColliderBits, get_global_index, first_piola_stress_neo_hookean, first_piola_stress_inviscid, ParticleParameters, IS_SOLID, IS_FLUID, USE_SAND_ALPHA};
import super::sand::sand_plasticity;

@group(0) @binding(0)
var<storage, read_write> masses: array<f32>;

@group(0) @binding(1)
var<storage, read_write> initial_volumes: array<f32>;

@group(0) @binding(2)
var<storage, read_write> parameters: array<ParticleParameters>;

@group(0) @binding(3)
var<storage, read_write> positions_and_collider_bits: array<PositionAndColliderBits>;

@group(0) @binding(4)
var<storage, read_write> position_gradients: array<mat3x3f>;

@group(0) @binding(5)
var<storage, read_write> velocities: array<vec3f>;

@group(0) @binding(6)
var<storage, read_write> velocity_gradients: array<mat3x3f>;

@group(0) @binding(7)
var<storage, read_write> tmp: array<mat4x4f>;

@group(0) @binding(8)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;
override GRID_NODE_SIZE: f32;
override TIME_STEP: f32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(subgroup_invocation_id) subgroup_invocation_id: u32,
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    if is_error() {
        return;
    }
    let scaling = TIME_STEP * 4. / (GRID_NODE_SIZE * GRID_NODE_SIZE);

    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index == 0 {
        set_shader_id();
    }
    if global_index >= arrayLength(&masses) {
        return;
    }

    let mass = masses[global_index];
    let initial_volume = initial_volumes[global_index];
    let parameter = parameters[global_index];
    let position: vec3f = positions_and_collider_bits[global_index].position;
    var position_gradient = position_gradients[global_index];
    let velocity = velocities[global_index];
    let velocity_gradient = velocity_gradients[global_index];

    var stress = mat3x3f();
    if (parameter.flags & IS_SOLID) > 0 {
        let mu = parameter.a;
        let lambda = parameter.b;
        if (parameter.flags & USE_SAND_ALPHA) > 0 {
            let sand_alpha = parameter.e;
            position_gradient = sand_plasticity(mu, lambda, sand_alpha, position_gradient);
            position_gradients[global_index] = position_gradient;
        }

        stress = first_piola_stress_neo_hookean(mu, lambda, position_gradient);
    }
    if (parameter.flags & IS_FLUID) > 0 {
        let exponent = parameter.a;
        let bulk_modulus = parameter.b;
        stress = first_piola_stress_inviscid(bulk_modulus, exponent, position_gradient);
    }

    let matrix_part = velocity_gradient * mass - stress * transpose(position_gradient) * scaling * initial_volume;
    let vector_part = velocity * mass;

    tmp[global_index] = mat4x4f(
        vec4f(matrix_part[0], position.x / GRID_NODE_SIZE),
        vec4f(matrix_part[1], position.y / GRID_NODE_SIZE),
        vec4f(matrix_part[2], position.z / GRID_NODE_SIZE),
        vec4f(vector_part, mass),
    );
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
