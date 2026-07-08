// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix3, Vector3};
use squishy_volumes_file_frame::{ParticleFlags, ParticleParameters};

#[derive(Default, Debug, Clone)]
pub struct Particles {
    pub sort_map: Vec<u32>,
    pub reverse_sort_map: Vec<u32>,

    pub flags: Vec<ParticleFlags>,

    pub parameters: Vec<ParticleParameters>,

    pub initial_positions: Vec<Vector3<f32>>,

    pub positions: Vec<Vector3<f32>>,
    pub position_gradients: Vec<Matrix3<f32>>,

    pub velocities: Vec<Vector3<f32>>,
    pub velocity_gradients: Vec<Matrix3<f32>>,

    pub elastic_energies: Vec<f32>,
    pub collider_bits: Vec<u32>,
}
