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
var<storage, read_write> position_gradients: array<mat3x3f>;

@group(0) @binding(2)
var<storage, read_write> particle_parameters: array<ParticleParameters>;

@group(0) @binding(3)
var<storage, read_write> stresses: array<mat3x3f>;

@group(0) @binding(4)
var<storage, read_write> energies: array<f32>;

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

    let position_gradient = position_gradients[global_index];
    let paramters = particle_parameters[global_index];

    let mu = paramters.a;
    let lambda = paramters.b;

    stresses[global_index] = first_piola_stress_neo_hookean(mu, lambda, position_gradient);
    energies[global_index] = elastic_energy_neo_hookean(mu, lambda, position_gradient);
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

struct ParticleParameters {
    flags: u32,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
}

fn invariant_2(position_gradient: mat3x3f) -> f32 {
    return dot(position_gradient[0], position_gradient[0]) +
        dot(position_gradient[1], position_gradient[1]) +
        dot(position_gradient[2], position_gradient[2]);
}

fn invariant_3(position_gradient: mat3x3f) -> f32 {
    return determinant(position_gradient);
}

fn elastic_energy_neo_hookean_by_invariants(
    mu: f32,
    lambda: f32,
    invariant_2: f32,
    invariant_3: f32,
) -> f32 {
    return mu / 2. * (invariant_2 - 3.) - mu * log(invariant_3) + lambda / 2. * log(invariant_3) * log(invariant_3);
}

fn elastic_energy_neo_hookean(
    mu: f32,
    lambda: f32,
    position_gradient: mat3x3f,
) -> f32 {
    return elastic_energy_neo_hookean_by_invariants(
        mu, lambda, invariant_2(position_gradient), invariant_3(position_gradient)
    );
}

fn partial_elastic_energy_neo_hookean_by_invariant_2(mu: f32) -> f32 {
    return mu / 2.;
}

fn partial_invariant_2_by_position_gradient(position_gradient: mat3x3f) -> mat3x3f {
    return 2. * position_gradient;
}

fn partial_elastic_energy_neo_hookean_by_invariant_3(mu: f32, lambda: f32, invariant_3: f32) -> f32 {
    return (lambda * log(invariant_3) - mu) / invariant_3;
}

fn partial_invariant_3_by_position_gradient(position_gradient: mat3x3f) -> mat3x3f {
    return mat3x3f(
        cross(position_gradient[1], position_gradient[2]),
        cross(position_gradient[2], position_gradient[0]),
        cross(position_gradient[0], position_gradient[1]),
    );
}

fn first_piola_stress_neo_hookean(
    mu: f32,
    lambda: f32,
    position_gradient: mat3x3f,
) -> mat3x3f {
    return partial_elastic_energy_neo_hookean_by_invariant_2(mu)
    * partial_invariant_2_by_position_gradient(position_gradient)
    + partial_elastic_energy_neo_hookean_by_invariant_3(
        mu,
        lambda,
        invariant_3(position_gradient),
    ) * partial_invariant_3_by_position_gradient(position_gradient);
}
