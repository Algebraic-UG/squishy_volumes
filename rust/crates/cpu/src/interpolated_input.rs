// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;

#[derive(Clone)]
pub struct InterpolatedInput {
    pub gravity: Vector3<f32>,

    pub particle_goal_positions: Vec<Vector3<f32>>,

    pub vertex_positions: Vec<Vector3<f32>>,
    pub vertex_normals: Vec<Vector3<f32>>,

    pub triangle_frictions: Vec<f32>,
    pub triangle_normals: Vec<Vector3<f32>>,
}
