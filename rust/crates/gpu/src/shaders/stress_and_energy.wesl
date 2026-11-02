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

import super::util::{Indirect, get_global_index, first_piola_stress_neo_hookean, first_piola_stress_inviscid, elastic_energy_neo_hookean, elastic_energy_inviscid, ParticleParameters, IS_SOLID, IS_FLUID, mu, lambda};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> position_gradients: array<mat3x3f>;

@group(0) @binding(2)
var<storage, read_write> particle_flags: array<u32>;

@group(0) @binding(3)
var<storage, read_write> particle_parameters: array<ParticleParameters>;

@group(0) @binding(4)
var<storage, read_write> stresses: array<mat3x3f>;

@group(0) @binding(5)
var<storage, read_write> energies: array<f32>;

@group(0) @binding(6)
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
    let flags = particle_flags[global_index];
    let parameters = particle_parameters[global_index];

    if (flags & IS_SOLID) > 0 {
        stresses[global_index] = first_piola_stress_neo_hookean(
            mu(parameters),
            lambda(parameters),
            position_gradient,
        );
        energies[global_index] = elastic_energy_neo_hookean(
            mu(parameters),
            lambda(parameters),
            position_gradient,
        );
        return;
    }

    if (flags & IS_FLUID) > 0 {
        stresses[global_index] = first_piola_stress_inviscid(
            parameters.bulk_modulus,
            parameters.exponent,
            position_gradient,
        );
        energies[global_index] = elastic_energy_inviscid(
            parameters.bulk_modulus,
            parameters.exponent,
            position_gradient,
        );
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
