// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use squishy_volumes_api::T;

pub fn kernel_linear(x: T) -> T {
    let x = x.abs();
    if x < 1. { 1. - x } else { 0. }
}

pub fn kernel_quadratic(x: T) -> T {
    let x = x.abs();
    if x < 1. / 2. {
        3. / 4. - x * x
    } else if x < 3. / 2. {
        1. / 2. * (3. / 2. - x) * (3. / 2. - x)
    } else {
        0.
    }
}

pub fn kernel_cubic(x: T) -> T {
    let x = x.abs();
    if x < 1. {
        1. / 2. * x * x * x - x * x + 2. / 3.
    } else if x < 2. {
        1. / 6. * (2. - x) * (2. - x) * (2. - x)
    } else {
        0.
    }
}

// XXX: changing this implies changing the shift for scatters.
pub const KERNEL_QUADRATIC_LENGTH: usize = 3;
pub const KERNEL_CUBIC_LENGTH: usize = 4;

pub fn position_to_shift_quadratic(position: &Vector3<T>, grid_node_size: T) -> Vector3<i32> {
    let normalized = position / grid_node_size;
    (normalized - Vector3::repeat(0.5)).map(|x| x.floor() as i32)
}

macro_rules! kernel_quadratic_unrolled {
    ($closure:expr) => {
        [
            ($closure)(Vector3::new(0, 0, 0)),
            ($closure)(Vector3::new(0, 0, 1)),
            ($closure)(Vector3::new(0, 0, 2)),
            ($closure)(Vector3::new(0, 1, 0)),
            ($closure)(Vector3::new(0, 1, 1)),
            ($closure)(Vector3::new(0, 1, 2)),
            ($closure)(Vector3::new(0, 2, 0)),
            ($closure)(Vector3::new(0, 2, 1)),
            ($closure)(Vector3::new(0, 2, 2)),
            ($closure)(Vector3::new(1, 0, 0)),
            ($closure)(Vector3::new(1, 0, 1)),
            ($closure)(Vector3::new(1, 0, 2)),
            ($closure)(Vector3::new(1, 1, 0)),
            ($closure)(Vector3::new(1, 1, 1)),
            ($closure)(Vector3::new(1, 1, 2)),
            ($closure)(Vector3::new(1, 2, 0)),
            ($closure)(Vector3::new(1, 2, 1)),
            ($closure)(Vector3::new(1, 2, 2)),
            ($closure)(Vector3::new(2, 0, 0)),
            ($closure)(Vector3::new(2, 0, 1)),
            ($closure)(Vector3::new(2, 0, 2)),
            ($closure)(Vector3::new(2, 1, 0)),
            ($closure)(Vector3::new(2, 1, 1)),
            ($closure)(Vector3::new(2, 1, 2)),
            ($closure)(Vector3::new(2, 2, 0)),
            ($closure)(Vector3::new(2, 2, 1)),
            ($closure)(Vector3::new(2, 2, 2)),
        ]
    };
}
pub(crate) use kernel_quadratic_unrolled;

pub fn kernel_quadratic_iter() -> impl Iterator<Item = Vector3<i32>> {
    (0..KERNEL_QUADRATIC_LENGTH as i32).flat_map(|i| {
        (0..KERNEL_QUADRATIC_LENGTH as i32).flat_map(move |j| {
            (0..KERNEL_QUADRATIC_LENGTH as i32).map(move |k| Vector3::new(i, j, k))
        })
    })
}
