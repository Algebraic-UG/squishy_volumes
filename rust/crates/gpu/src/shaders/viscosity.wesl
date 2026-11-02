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

import super::util::{Indirect, get_global_index, ParticleParameters, USE_VISCOSITY, trace, from_diagonal_element};

@group(0) @binding(0)
var<storage, read_write> particle_parameters: array<ParticleParameters>;

@group(0) @binding(1)
var<storage, read_write> particle_velocity_gradients: array<mat3x3f>;

@group(0) @binding(2)
var<storage, read_write> particle_stresses: array<mat3x3f>;

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

    let parameters = particle_parameters[global_index];
    let velocity_gradient = particle_velocity_gradients[global_index];

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
    let d = 0.5 * (velocity_gradient + transpose(velocity_gradient));
    let divergence = trace(velocity_gradient);
    let deviatoric_d = d - from_diagonal_element(divergence / 3.);

    return 2. * dynamic_viscosity * deviatoric_d + bulk_viscosity * from_diagonal_element(divergence);
}

