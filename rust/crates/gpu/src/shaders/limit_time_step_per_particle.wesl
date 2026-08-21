// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{PositionAndColliderBits, get_global_index, first_piola_stress_neo_hookean, first_piola_stress_inviscid, ParticleParameters, IS_SOLID, IS_FLUID, USE_SAND_ALPHA, USE_VISCOSITY, INVERSE_EPS, PARTICLE_CLOSE_TO_INVERTED, FAILED, TOMBSTONED, TimeStepLimits, SINGULAR_VALUE_SEPARATION, first_piola_stress_neo_hookean_svd_in_diagonal_space, first_piola_stress_inviscid_svd_in_diagonal_space, second_derivative_neo_hookean_svd_in_diagonal_space, second_derivative_inviscid_svd_in_diagonal_space, F32_MIN, partial_elastic_energy_inviscid_by_invariant_3, double_partial_elastic_energy_inviscid_by_invariant_3, mat3x3f_max_elem, mat3x3f_abs};
import super::external::wgebra::svd3::{Svd, svd, recompose};

@group(0) @binding(0)
var<storage, read_write> flags: array<u32>;

@group(0) @binding(1)
var<storage, read_write> parameters: array<ParticleParameters>;

@group(0) @binding(2)
var<storage, read_write> position_gradients: array<mat3x3f>;

@group(0) @binding(3)
var<storage, read_write> velocities: array<vec3f>;

@group(0) @binding(4)
var<storage, read_write> velocity_gradients: array<mat3x3f>;

@group(0) @binding(5)
var<storage, read_write> time_step_limits: array<TimeStepLimits>;

@group(0) @binding(6)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;
override GRID_NODE_SIZE: f32;

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
    if global_index >= arrayLength(&flags) {
        return;
    }

    let flag = flags[global_index];
    if (flag & TOMBSTONED) != 0 {
        return;
    }

    let parameter = parameters[global_index];
    var position_gradient = position_gradients[global_index];
    let velocity = velocities[global_index];
    let velocity_gradient = velocity_gradients[global_index];

    var limits: TimeStepLimits;
    limits.time_step_by_sound = limit_time_step_by_speed_of_sound(flag, parameter, position_gradient);
    limits.time_step_by_isolated = limit_time_step_by_isolated(flag, parameter, position_gradient);
    limits.time_step_by_velocity = limit_time_step_by_velocity(velocity);
    limits.time_step_by_deformation = limit_time_step_by_deformation(velocity_gradient);

    time_step_limits[global_index] = limits;
}

fn limit_time_step_by_speed_of_sound(flag: u32, parameter: ParticleParameters, position_gradient: mat3x3f) -> f32 {
    let s = svd(position_gradient).S;

    let j = s.x * s.y * s.z;

    let xy_close = abs(s.x - s.y) < SINGULAR_VALUE_SEPARATION;
    let yz_close = abs(s.y - s.z) < SINGULAR_VALUE_SEPARATION;
    let zx_close = abs(s.z - s.x) < SINGULAR_VALUE_SEPARATION;

    var first = vec3f(0.);
    var second = mat3x3f();

    if (flag & IS_SOLID) > 0 {
        first = first_piola_stress_neo_hookean_svd_in_diagonal_space(parameter.mu, parameter.lambda, s);
        second = second_derivative_neo_hookean_svd_in_diagonal_space(parameter.mu, parameter.lambda, s);
    }

    if (flag & IS_FLUID) > 0 {
        first = first_piola_stress_inviscid_svd_in_diagonal_space(parameter.bulk_modulus, parameter.exponent, s);
        second = second_derivative_inviscid_svd_in_diagonal_space(parameter.bulk_modulus, parameter.exponent, s);
    }

    var kappa = 0.;
    kappa = max(kappa, s.x * s.x * second[0].x);
    kappa = max(kappa, s.y * s.y * second[1].y);
    kappa = max(kappa, s.z * s.z * second[2].z);
    if xy_close {
        kappa = max(kappa, s.x * s.y * (first.x + s.x * second[0].x - s.y * second[0].y) / 2. / s.x);
    } else {
        kappa = max(kappa, s.x * s.y * (s.x * first.x - s.y * first.y) / (s.x * s.x - s.y * s.y));
    }

    if yz_close {
        kappa = max(kappa, s.y * s.z * (first.y + s.y * second[1].y - s.z * second[1].z) / 2. / s.y);
    } else {
        kappa = max(kappa, s.y * s.z * (s.y * first.y - s.z * first.z) / (s.y * s.y - s.z * s.z));
    }

    if zx_close {
        kappa = max(kappa, s.z * s.x * (first.z + s.z * second[2].z - s.x * second[2].x) / 2. / s.z);
    } else {
        kappa = max(kappa, s.z * s.x * (s.z * first.z - s.x * first.x) / (s.z * s.z - s.x * s.x));
    }

    kappa /= j;

    let initial_density = parameter.mass / parameter.initial_volume;
    let current_density = initial_density / j;

    let c = sqrt(kappa / current_density);

    return GRID_NODE_SIZE / c;
}

fn limit_time_step_by_isolated(flag: u32, parameters: ParticleParameters, position_gradient: mat3x3f) -> f32 {
    if (flag & IS_SOLID) > 0 {
        // Stability analysis of explicit MPM, Technical document 3.12
        let xi = 3. / GRID_NODE_SIZE / GRID_NODE_SIZE;
        const R: f32 = 1.; // APIC & CPIC
        const K: f32 = 1.; // CPIC
        const D: f32 = 3.; // 3D
        return sqrt(parameters.mass / (parameters.initial_volume * xi * (R - K / 2.) * (parameters.mu + D / 2. * parameters.lambda)));
    }

    if (flag & IS_FLUID) > 0 {
        // Effective time step restrictions for explicit MPM simulation,
        // Technical document "Simple bounds"
        let initial_density = parameters.mass / parameters.initial_volume;
        let j = determinant(position_gradient);
        const K: f32 = 6.; // quadratic splines
        const D: f32 = 3.; // 3D
        let first = partial_elastic_energy_inviscid_by_invariant_3(parameters.bulk_modulus, parameters.exponent, j);
        if abs(j - 1.) > SINGULAR_VALUE_SEPARATION {
            return GRID_NODE_SIZE / j * sqrt(initial_density * (j - 1.) / (K * first * D));
        }

        let second = double_partial_elastic_energy_inviscid_by_invariant_3(parameters.bulk_modulus, parameters.exponent, j);

        return sqrt(GRID_NODE_SIZE * (initial_density / (K * second * D)));
    }

    return 0.;
}

fn limit_time_step_by_velocity(velocity: vec3f) -> f32 {
    return GRID_NODE_SIZE * 0.5 / max(length(velocity), GRID_NODE_SIZE * 0.5);
}

fn limit_time_step_by_deformation(velocity_gradient: mat3x3f) -> f32 {
    const DELTA: f32 = 0.2;
    return DELTA / max(mat3x3f_max_elem(mat3x3f_abs(velocity_gradient)), 1e-8);
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
