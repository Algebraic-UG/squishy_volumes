// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct InputConsts {
    pub grid_node_size: T,
    pub simulation_scale: T,
    pub frames_per_second: u32,
    pub domain_min: Vector3<T>,
    pub domain_max: Vector3<T>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum InputObjectType {
    Particles,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct InputObject {
    pub name: String,
    pub ty: InputObjectType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct InputHeader {
    pub consts: InputConsts,
    pub objects: Vec<InputObject>,
}
