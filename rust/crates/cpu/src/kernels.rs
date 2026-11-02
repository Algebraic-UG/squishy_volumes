// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use nalgebra::Vector3;

#[allow(unused)]
pub fn kernel_linear(x: f32) -> f32 {
    let x = x.abs();
    if x < 1. { 1. - x } else { 0. }
}

pub fn kernel_quadratic(x: f32) -> f32 {
    let x = x.abs();
    if x < 1. / 2. {
        3. / 4. - x * x
    } else if x < 3. / 2. {
        1. / 2. * (3. / 2. - x) * (3. / 2. - x)
    } else {
        0.
    }
}

#[allow(unused)]
pub fn kernel_cubic(x: f32) -> f32 {
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

#[allow(unused)]
pub const KERNEL_CUBIC_LENGTH: usize = 4;

pub fn position_to_shift_quadratic(position: &Vector3<f32>, grid_node_size: f32) -> Vector3<i32> {
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
