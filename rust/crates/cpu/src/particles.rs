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
use squishy_volumes_util::{ParticleFlags, ParticleParameters};

#[derive(Default, Debug, Clone)]
pub struct Particles {
    pub sort_map: Vec<u32>,
    pub reverse_sort_map: Vec<u32>,

    pub flags: Vec<ParticleFlags>,

    pub parameters: Vec<ParticleParameters>,

    pub initial_positions: Vec<Vector3<f32>>,
    pub goal_positions: Vec<Vector3<f32>>,

    pub positions: Vec<Vector3<f32>>,
    pub position_gradients: Vec<Matrix3<f32>>,

    pub velocities: Vec<Vector3<f32>>,
    pub velocity_gradients: Vec<Matrix3<f32>>,

    pub elastic_energies: Vec<f32>,
    pub collider_bits: Vec<u32>,
}
