// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;

use super::Mutex;

//#[derive(Debug, Clone, Serialize, Deserialize)]
//pub struct Boundary {
//    // fixed in one time step
//    pub normal: Vector3<T>,
//    pub collider_value: T,
//
//    // change in implicit solving
//    pub condition_value: T,
//    pub dual_variable: T,
//}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GridKey {
    pub node_id: Vector3<i32>,
    pub collider_bits: u32,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct GridMomentum {
    pub map: FxHashMap<GridKey, usize>,

    pub keys: Vec<GridKey>,

    pub contributors: Vec<Mutex<Vec<usize>>>,

    pub masses: Vec<T>,
    pub velocities: Vec<Vector3<T>>,
    // TODO: these are not needed for explicit integration
    //pub reference_velocities: Vec<Vector3<T>>,
    //pub newton_direction: Vec<Vector3<T>>,

    //pub boundaries: Vec<Option<Boundary>>,

    //pub residual: Vec<Vector3<T>>,

    //pub cg_direction: Vec<Vector3<T>>,
    //pub cg_conjugated: Vec<Vector3<T>>,
}
