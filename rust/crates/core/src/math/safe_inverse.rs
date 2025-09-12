// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix2, Matrix3, Matrix4};
use squishy_volumes_api::T;

use super::INVERSE_EPS;

pub trait SafeInverse: Sized {
    fn safe_inverse(&self) -> Option<Self>;
}

impl SafeInverse for Matrix2<T> {
    fn safe_inverse(&self) -> Option<Self> {
        if self.determinant().abs() < INVERSE_EPS {
            return None;
        }
        self.try_inverse()
    }
}

impl SafeInverse for Matrix3<T> {
    fn safe_inverse(&self) -> Option<Self> {
        if self.determinant().abs() < INVERSE_EPS {
            return None;
        }
        self.try_inverse()
    }
}

impl SafeInverse for Matrix4<T> {
    fn safe_inverse(&self) -> Option<Self> {
        if self.determinant().abs() < INVERSE_EPS {
            return None;
        }
        self.try_inverse()
    }
}
