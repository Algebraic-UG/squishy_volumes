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

use nalgebra::{Matrix3, Vector3};
use squishy_volumes_util::{
    ParticleFlags, ParticleParameters, cauchy_stress_general_viscosity,
    first_piola_stress_inviscid, first_piola_stress_neo_hookean,
};

use crate::{kernel_quadratic, kernel_quadratic_iter, position_to_shift_quadratic};

pub fn quadratic_inverse_inertia(grid_node_size: f32) -> f32 {
    4. / grid_node_size / grid_node_size
}

pub fn node_weight(
    node_id: &Vector3<i32>,
    position: &Vector3<f32>,
    grid_node_size: f32,
) -> (f32, Vector3<f32>) {
    let to_grid_node_normalized = node_id.map(|x| x as f32) - position / grid_node_size;
    (
        to_grid_node_normalized.map(kernel_quadratic).product(),
        to_grid_node_normalized * grid_node_size,
    )
}

pub fn quadratic_stencil(
    position: &Vector3<f32>,
    grid_node_size: f32,
) -> impl Iterator<Item = Vector3<i32>> {
    let shift = position_to_shift_quadratic(position, grid_node_size);
    kernel_quadratic_iter().map(move |offset| shift + offset)
}

pub fn kirchhoff_stress(
    flags: ParticleFlags,
    parameters: &ParticleParameters,
    position_gradient: &Matrix3<f32>,
    velocity_gradient: &Matrix3<f32>,
) -> Matrix3<f32> {
    let first_piola_stress = if flags.contains(ParticleFlags::IS_SOLID) {
        first_piola_stress_neo_hookean(parameters.mu(), parameters.lambda(), position_gradient)
    } else if flags.contains(ParticleFlags::IS_FLUID) {
        first_piola_stress_inviscid(
            parameters.bulk_modulus,
            parameters.exponent,
            position_gradient,
        )
    } else {
        unreachable!()
    };
    let elastic_stress = first_piola_stress * position_gradient.transpose();
    if !flags.contains(ParticleFlags::USE_VISCOSITY) {
        return elastic_stress;
    }
    let viscous_stress = cauchy_stress_general_viscosity(
        parameters.viscosity_dynamic,
        parameters.viscosity_bulk,
        velocity_gradient,
    ) * position_gradient.determinant();
    elastic_stress + viscous_stress
}

pub fn affine_momentum(
    to_grid_node: &Vector3<f32>,
    mass: f32,
    velocity: &Vector3<f32>,
    velocity_gradient: &Matrix3<f32>,
) -> Vector3<f32> {
    (velocity + velocity_gradient * to_grid_node) * mass
}

pub struct StressImpulse<'a> {
    pub kirchhoff_stress: &'a Matrix3<f32>,
    pub initial_volume: f32,
    pub time_step: f32,
    pub grid_node_size: f32,
}

impl StressImpulse<'_> {
    pub fn towards(&self, to_grid_node: &Vector3<f32>) -> Vector3<f32> {
        self.kirchhoff_stress
            * to_grid_node
            * (self.time_step
                * quadratic_inverse_inertia(self.grid_node_size)
                * self.initial_volume)
    }
}

pub fn gather_from_nodes(
    nodes: impl Iterator<Item = (f32, Vector3<f32>, Vector3<f32>)>,
    grid_node_size: f32,
) -> (Vector3<f32>, Matrix3<f32>) {
    let mut velocity = Vector3::zeros();
    let mut velocity_gradient = Matrix3::zeros();
    for (weight, to_grid_node, grid_velocity) in nodes {
        velocity += grid_velocity * weight;
        velocity_gradient += (grid_velocity * weight) * to_grid_node.transpose();
    }
    (
        velocity,
        velocity_gradient * quadratic_inverse_inertia(grid_node_size),
    )
}
