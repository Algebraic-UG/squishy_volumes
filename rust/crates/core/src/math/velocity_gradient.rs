// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix3, Vector3};
use squishy_volumes_api::T;

pub fn velocity_gradient_from_angular_velocity(angular_velocity: &Vector3<T>) -> Matrix3<T> {
    Matrix3::from_columns(&[
        Vector3::new(0., angular_velocity.z, -angular_velocity.y),
        Vector3::new(-angular_velocity.z, 0., angular_velocity.x),
        Vector3::new(angular_velocity.y, -angular_velocity.x, 0.),
    ])
}
