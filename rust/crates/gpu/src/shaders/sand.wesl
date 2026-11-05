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

import super::util::{Indirect, get_global_index, ParticleParameters, IS_SOLID, USE_SAND_ALPHA, mu, lambda};
import super::external::wgebra::svd3::{Svd, svd, recompose};

@group(0) @binding(0)
var<storage, read_write> particle_parameters: array<ParticleParameters>;

@group(0) @binding(1)
var<storage, read_write> particle_position_gradients: array<mat3x3f>;

@group(0) @binding(2)
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

    particle_position_gradients[global_index] = sand_plasticity(
        mu(parameters),
        lambda(parameters),
        parameters.sand_alpha,
        particle_position_gradients[global_index],
    );
}

fn sand_plasticity(
    mu: f32,
    lambda: f32,
    sand_alpha: f32,
    position_gradient: mat3x3f
) -> mat3x3f {
    var svd = svd(position_gradient);
    let hencky_strain = log(svd.S);
    let volumetric_strain = hencky_strain.x + hencky_strain.y + hencky_strain.z;

    let projects_to_cone_tip = mu <= 0. || volumetric_strain >= 0.;
    if projects_to_cone_tip {
        return svd.U * svd.Vt;
    }

    let deviatoric_strain = hencky_strain - vec3f(volumetric_strain / 3.);
    let deviatoric_strain_norm = length(deviatoric_strain);
    let is_pure_compression = deviatoric_strain_norm == 0.;
    if is_pure_compression {
        return position_gradient;
    }

    let plastic_multiplier = deviatoric_strain_norm
        + (3. * lambda + 2. * mu) / (2. * mu) * volumetric_strain * sand_alpha;
    let is_inside_cone = plastic_multiplier <= 0.;
    if is_inside_cone {
        return position_gradient;
    }

    let projected_strain = hencky_strain - plastic_multiplier / deviatoric_strain_norm * deviatoric_strain;
    svd.S = exp(projected_strain);
    return recompose(svd);
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
