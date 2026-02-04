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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BulkData {
    F32(Vec<f32>),
    I32(Vec<i32>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InputFrame {
    pub gravity: Vector3<T>,
    pub bulk: BTreeMap<String, BulkData>,
}
