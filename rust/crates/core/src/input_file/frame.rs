// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::BTreeMap;

use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct ParticlesInput {
    pub flags: Vec<i32>,
    pub transforms: Vec<f32>,
    pub sizes: Vec<f32>,
    pub densities: Vec<f32>,
    pub youngs_moduluses: Vec<f32>,
    pub poissons_ratios: Vec<f32>,
    pub initial_positions: Vec<f32>,
    pub initial_velocity: Vec<f32>,
    pub viscosity_dynamic: Vec<f32>,
    pub viscosity_bulk: Vec<f32>,
    pub exponent: Vec<f32>,
    pub bulk_modulus: Vec<f32>,
    pub sand_alpha: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InputFrame {
    pub gravity: Vector3<T>,
    pub particles_input: BTreeMap<String, ParticlesInput>,
}
