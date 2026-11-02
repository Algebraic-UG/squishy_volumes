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
