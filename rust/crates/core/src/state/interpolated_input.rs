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

#[derive(Clone, Serialize, Deserialize)]
pub struct InterpolatedInput {
    pub gravity: Vector3<T>,

    pub particle_goal_positions: Vec<Vector3<T>>,

    pub vertex_positions: Vec<Vector3<T>>,
    pub vertex_normals: Vec<Vector3<T>>,

    pub triangle_frictions: Vec<T>,
    pub triangle_normals: Vec<Vector3<T>>,
}
