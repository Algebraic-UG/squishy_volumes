// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix3, Matrix4, Vector3, Vector4};

pub trait Flat3 {
    fn flat(&self) -> [f32; 3];
}

impl Flat3 for Vector3<f32> {
    fn flat(&self) -> [f32; 3] {
        self.data.0[0]
    }
}

#[allow(unused)]
pub trait Flat4 {
    fn flat(&self) -> [f32; 4];
}

impl Flat4 for Vector4<f32> {
    fn flat(&self) -> [f32; 4] {
        self.data.0[0]
    }
}

pub trait Flat9 {
    fn flat(&self) -> [f32; 9];
}

impl Flat9 for Matrix3<f32> {
    fn flat(&self) -> [f32; 9] {
        [
            self.data.0[0][0],
            self.data.0[0][1],
            self.data.0[0][2],
            self.data.0[1][0],
            self.data.0[1][1],
            self.data.0[1][2],
            self.data.0[2][0],
            self.data.0[2][1],
            self.data.0[2][2],
        ]
    }
}

pub trait Flat16 {
    fn flat(&self) -> [f32; 16];
}

impl Flat16 for Matrix4<f32> {
    fn flat(&self) -> [f32; 16] {
        [
            self.data.0[0][0],
            self.data.0[0][1],
            self.data.0[0][2],
            self.data.0[0][3],
            self.data.0[1][0],
            self.data.0[1][1],
            self.data.0[1][2],
            self.data.0[1][3],
            self.data.0[2][0],
            self.data.0[2][1],
            self.data.0[2][2],
            self.data.0[2][3],
            self.data.0[3][0],
            self.data.0[3][1],
            self.data.0[3][2],
            self.data.0[3][3],
        ]
    }
}
