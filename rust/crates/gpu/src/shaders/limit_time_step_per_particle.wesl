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

import super::util::{PositionAndColliderBits, get_global_index, first_piola_stress_neo_hookean, first_piola_stress_inviscid, ParticleParameters, IS_SOLID, IS_FLUID, USE_SAND_ALPHA, USE_VISCOSITY, INVERSE_EPS, PARTICLE_CLOSE_TO_INVERTED, FAILED, TOMBSTONED, TimeStepLimits, SINGULAR_VALUE_SEPARATION, first_piola_stress_neo_hookean_svd_in_diagonal_space, first_piola_stress_inviscid_svd_in_diagonal_space, second_derivative_neo_hookean_svd_in_diagonal_space, second_derivative_inviscid_svd_in_diagonal_space, F32_MIN, partial_elastic_energy_inviscid_by_invariant_3, double_partial_elastic_energy_inviscid_by_invariant_3, mat3x3f_max_elem, mat3x3f_abs, F32_MAX, lambda, mu};
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
        time_step_limits[global_index].time_step_by_sound = F32_MAX;
        time_step_limits[global_index].time_step_by_isolated = F32_MAX;
        time_step_limits[global_index].time_step_by_velocity = F32_MAX;
        time_step_limits[global_index].time_step_by_deformation = F32_MAX;
        return;
    }

    let parameter = parameters[global_index];
    let principal_stretches = svd(position_gradients[global_index]).S;
    let velocity = velocities[global_index];
    let velocity_gradient = velocity_gradients[global_index];

    var limits: TimeStepLimits;
    limits.time_step_by_sound = limit_time_step_by_speed_of_sound(flag, parameter, principal_stretches);
    limits.time_step_by_isolated = limit_time_step_by_isolated(flag, parameter, principal_stretches);
    limits.time_step_by_velocity = limit_time_step_by_velocity(velocity);
    limits.time_step_by_deformation = limit_time_step_by_deformation(velocity_gradient);

    time_step_limits[global_index] = limits;
}

fn limit_time_step_by_speed_of_sound(flag: u32, parameter: ParticleParameters, principal_stretches: vec3f) -> f32 {
    let s = principal_stretches;

    let j = s.x * s.y * s.z;

    let xy_close = abs(s.x - s.y) < SINGULAR_VALUE_SEPARATION;
    let yz_close = abs(s.y - s.z) < SINGULAR_VALUE_SEPARATION;
    let zx_close = abs(s.z - s.x) < SINGULAR_VALUE_SEPARATION;

    var first = vec3f(0.);
    var second = mat3x3f();

    if (flag & IS_SOLID) > 0 {
        first = first_piola_stress_neo_hookean_svd_in_diagonal_space(mu(parameter), lambda(parameter), s);
        second = second_derivative_neo_hookean_svd_in_diagonal_space(mu(parameter), lambda(parameter), s);
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

    let has_no_stiffness = kappa <= 0.;
    if has_no_stiffness {
        return F32_MAX;
    }

    let current_density = parameter.density / j;

    let c = sqrt(kappa / current_density);

    return GRID_NODE_SIZE / c;
}

fn limit_time_step_by_isolated(flag: u32, parameters: ParticleParameters, principal_stretches: vec3f) -> f32 {
    let inverse_inertia = 4. / GRID_NODE_SIZE / GRID_NODE_SIZE;
    var elastic_shear = 0.;
    var elastic_volume = 0.;
    if (flag & IS_SOLID) > 0 {
        let per_density = inverse_inertia / parameters.density;
        let largest_stretch = max(max(abs(principal_stretches.x), abs(principal_stretches.y)), abs(principal_stretches.z));
        let stretched_mu = mu(parameters) * max(largest_stretch * largest_stretch, 1.);
        elastic_shear = 2. * stretched_mu * per_density;
        elastic_volume = (2. * stretched_mu + 3. * lambda(parameters)) * per_density;
    } else if (flag & IS_FLUID) > 0 {
        let time_step = limit_time_step_by_isolated_fluid(parameters, principal_stretches);
        elastic_volume = 4. / time_step / time_step;
    }

    return min(
        limit_time_step_of_oscillator(elastic_shear),
        limit_time_step_of_oscillator(elastic_volume),
    );
}

fn limit_time_step_by_isolated_fluid(parameters: ParticleParameters, principal_stretches: vec3f) -> f32 {
    // Effective time step restrictions for explicit MPM simulation,
    // Technical document "Simple bounds"
    if parameters.bulk_modulus <= 0. {
        return F32_MAX;
    }
    let j = principal_stretches.x * principal_stretches.y * principal_stretches.z;
    const K: f32 = 6.; // quadratic splines
    const D: f32 = 3.; // 3D
    let first = partial_elastic_energy_inviscid_by_invariant_3(parameters.bulk_modulus, parameters.exponent, j);
    if abs(j - 1.) > SINGULAR_VALUE_SEPARATION {
        return GRID_NODE_SIZE / j * sqrt(parameters.density * (j - 1.) / (K * first * D));
    }

    let second = double_partial_elastic_energy_inviscid_by_invariant_3(parameters.bulk_modulus, parameters.exponent, j);

    return GRID_NODE_SIZE * sqrt(parameters.density / (K * second * D));
}

fn limit_time_step_of_oscillator(stiffness: f32) -> f32 {
    if stiffness <= 0. {
        return F32_MAX;
    }
    return 2. / sqrt(stiffness);
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
