// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use squishy_volumes_api::T;

pub fn check_shifted_quadratic(shifted: Vector3<T>) -> bool {
    shifted.x >= 0.5
        && shifted.x <= 1.5
        && shifted.y >= 0.5
        && shifted.y <= 1.5
        && shifted.z >= 0.5
        && shifted.z <= 1.5
}

#[allow(unused)]
pub fn check_shifted_cubic(shifted: Vector3<T>) -> bool {
    shifted.x >= 1.
        && shifted.x <= 2.
        && shifted.y >= 1.
        && shifted.y <= 2.
        && shifted.z >= 1.
        && shifted.z <= 2.
}
