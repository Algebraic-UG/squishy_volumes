// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use squishy_volumes_util::{ParticleFlags, ParticleParameters};

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Particles {
    pub flags: Vec<ParticleFlags>,

    pub parameters: Vec<ParticleParameters>,

    pub elastic_energies: Vec<f32>,

    pub collider_bits: Vec<u32>,

    pub positions: Vec<[f32; 3]>,
    pub position_gradients: Vec<[[f32; 3]; 3]>,

    pub velocities: Vec<[f32; 3]>,
    pub velocity_gradients: Vec<[[f32; 3]; 3]>,

    pub initial_positions: Vec<[f32; 3]>,
}
