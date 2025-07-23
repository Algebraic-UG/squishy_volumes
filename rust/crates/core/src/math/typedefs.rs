// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{ArrayStorage, Matrix, U1, U9};

pub type Vector9<T> = Matrix<T, U9, U1, ArrayStorage<T, 9, 1>>;
pub type Matrix9<T> = Matrix<T, U9, U9, ArrayStorage<T, 9, 9>>;
