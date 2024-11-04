// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use blended_mpm_api::T;
use nalgebra::{Matrix2, Matrix3, Vector2, Vector3};

#[allow(unused)]
pub fn basis_from_direction_2d(dir: Vector2<T>) -> Matrix2<T> {
    debug_assert!((dir.norm() - 1.).abs() < 1e-3);
    Matrix2::from_columns(&[dir, Vector2::new(-dir.y, dir.x)])
}

pub fn basis_from_direction_3d(dir: Vector3<T>) -> Matrix3<T> {
    debug_assert!((dir.norm() - 1.).abs() < 1e-3);
    let col0 = dir;
    let col1 = col0
        .cross(&if dir.x.abs() < 0.9 {
            Vector3::x()
        } else {
            Vector3::y()
        })
        .normalize();
    let col2 = col0.cross(&col1);

    Matrix3::from_columns(&[col0, col1, col2])
}
