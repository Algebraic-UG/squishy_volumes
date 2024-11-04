// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use blended_mpm_api::T;
use fxhash::FxHashMap;
use nalgebra::{Matrix3, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParticleParameters {
    Solid { mu: T, lambda: T },
    Fluid { exponent: i32, bulk_modulus: T },
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Particles {
    pub sort_map: Vec<usize>,
    pub reverse_sort_map: Vec<usize>,

    pub parameters: Vec<ParticleParameters>,

    pub masses: Vec<T>,
    pub initial_volumes: Vec<T>,

    pub positions: Vec<Vector3<T>>,
    pub position_gradients: Vec<Matrix3<T>>,

    pub velocities: Vec<Vector3<T>>,
    pub velocity_gradients: Vec<Matrix3<T>>,

    pub elastic_energies: Vec<T>,
    pub collider_insides: Vec<FxHashMap<usize, bool>>,

    pub trial_position_gradients: Vec<Matrix3<T>>,
    pub action_matrices: Vec<Matrix3<T>>,
}
