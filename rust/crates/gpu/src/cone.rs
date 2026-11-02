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

use nalgebra::{Vector3, Vector4};
use squishy_volumes_mesh_util::Triangle;

pub fn vertices() -> Vec<Vector4<f32>> {
    #[allow(clippy::excessive_precision, clippy::approx_constant)]
    [
        Vector3::new(0.000000, -1.000000, -1.000000),
        Vector3::new(0.195090, -1.000000, -0.980785),
        Vector3::new(0.382683, -1.000000, -0.923880),
        Vector3::new(0.555570, -1.000000, -0.831470),
        Vector3::new(0.707107, -1.000000, -0.707107),
        Vector3::new(0.831470, -1.000000, -0.555570),
        Vector3::new(0.923880, -1.000000, -0.382683),
        Vector3::new(0.980785, -1.000000, -0.195090),
        Vector3::new(1.000000, -1.000000, 0.000000),
        Vector3::new(0.980785, -1.000000, 0.195090),
        Vector3::new(0.923880, -1.000000, 0.382683),
        Vector3::new(0.831470, -1.000000, 0.555570),
        Vector3::new(0.707107, -1.000000, 0.707107),
        Vector3::new(0.555570, -1.000000, 0.831470),
        Vector3::new(0.382683, -1.000000, 0.923880),
        Vector3::new(0.195090, -1.000000, 0.980785),
        Vector3::new(0.000000, -1.000000, 1.000000),
        Vector3::new(-0.195090, -1.000000, 0.980785),
        Vector3::new(-0.382683, -1.000000, 0.923880),
        Vector3::new(-0.555570, -1.000000, 0.831470),
        Vector3::new(-0.707107, -1.000000, 0.707107),
        Vector3::new(-0.831470, -1.000000, 0.555570),
        Vector3::new(-0.923880, -1.000000, 0.382683),
        Vector3::new(-0.980785, -1.000000, 0.195090),
        Vector3::new(-1.000000, -1.000000, 0.000000),
        Vector3::new(-0.980785, -1.000000, -0.195090),
        Vector3::new(-0.923880, -1.000000, -0.382683),
        Vector3::new(-0.831470, -1.000000, -0.555570),
        Vector3::new(-0.707107, -1.000000, -0.707107),
        Vector3::new(-0.555570, -1.000000, -0.831470),
        Vector3::new(-0.382683, -1.000000, -0.923880),
        Vector3::new(-0.195090, -1.000000, -0.980785),
        Vector3::new(0.000000, 1.000000, 0.000000),
    ]
    .into_iter()
    .map(|v| v.push(0.))
    .collect()
}

pub fn triangles() -> Vec<Triangle> {
    vec![
        Triangle { a: 31, b: 0, c: 1 },
        Triangle { a: 1, b: 2, c: 3 },
        Triangle { a: 3, b: 4, c: 5 },
        Triangle { a: 5, b: 6, c: 7 },
        Triangle { a: 7, b: 8, c: 9 },
        Triangle { a: 9, b: 10, c: 11 },
        Triangle {
            a: 11,
            b: 12,
            c: 13,
        },
        Triangle {
            a: 13,
            b: 14,
            c: 15,
        },
        Triangle {
            a: 15,
            b: 16,
            c: 17,
        },
        Triangle {
            a: 17,
            b: 18,
            c: 19,
        },
        Triangle {
            a: 19,
            b: 20,
            c: 21,
        },
        Triangle {
            a: 21,
            b: 22,
            c: 23,
        },
        Triangle {
            a: 23,
            b: 24,
            c: 25,
        },
        Triangle {
            a: 25,
            b: 26,
            c: 27,
        },
        Triangle {
            a: 27,
            b: 28,
            c: 29,
        },
        Triangle {
            a: 29,
            b: 30,
            c: 31,
        },
        Triangle { a: 31, b: 1, c: 7 },
        Triangle { a: 1, b: 3, c: 7 },
        Triangle { a: 3, b: 5, c: 7 },
        Triangle { a: 7, b: 9, c: 15 },
        Triangle { a: 9, b: 11, c: 15 },
        Triangle {
            a: 11,
            b: 13,
            c: 15,
        },
        Triangle {
            a: 15,
            b: 17,
            c: 23,
        },
        Triangle {
            a: 17,
            b: 19,
            c: 23,
        },
        Triangle {
            a: 19,
            b: 21,
            c: 23,
        },
        Triangle {
            a: 23,
            b: 25,
            c: 31,
        },
        Triangle {
            a: 25,
            b: 27,
            c: 31,
        },
        Triangle {
            a: 27,
            b: 29,
            c: 31,
        },
        Triangle { a: 31, b: 7, c: 15 },
        Triangle {
            a: 15,
            b: 23,
            c: 31,
        },
        Triangle { a: 0, b: 32, c: 1 },
        Triangle { a: 1, b: 32, c: 2 },
        Triangle { a: 2, b: 32, c: 3 },
        Triangle { a: 3, b: 32, c: 4 },
        Triangle { a: 4, b: 32, c: 5 },
        Triangle { a: 5, b: 32, c: 6 },
        Triangle { a: 6, b: 32, c: 7 },
        Triangle { a: 7, b: 32, c: 8 },
        Triangle { a: 8, b: 32, c: 9 },
        Triangle { a: 9, b: 32, c: 10 },
        Triangle {
            a: 10,
            b: 32,
            c: 11,
        },
        Triangle {
            a: 11,
            b: 32,
            c: 12,
        },
        Triangle {
            a: 12,
            b: 32,
            c: 13,
        },
        Triangle {
            a: 13,
            b: 32,
            c: 14,
        },
        Triangle {
            a: 14,
            b: 32,
            c: 15,
        },
        Triangle {
            a: 15,
            b: 32,
            c: 16,
        },
        Triangle {
            a: 16,
            b: 32,
            c: 17,
        },
        Triangle {
            a: 17,
            b: 32,
            c: 18,
        },
        Triangle {
            a: 18,
            b: 32,
            c: 19,
        },
        Triangle {
            a: 19,
            b: 32,
            c: 20,
        },
        Triangle {
            a: 20,
            b: 32,
            c: 21,
        },
        Triangle {
            a: 21,
            b: 32,
            c: 22,
        },
        Triangle {
            a: 22,
            b: 32,
            c: 23,
        },
        Triangle {
            a: 23,
            b: 32,
            c: 24,
        },
        Triangle {
            a: 24,
            b: 32,
            c: 25,
        },
        Triangle {
            a: 25,
            b: 32,
            c: 26,
        },
        Triangle {
            a: 26,
            b: 32,
            c: 27,
        },
        Triangle {
            a: 27,
            b: 32,
            c: 28,
        },
        Triangle {
            a: 28,
            b: 32,
            c: 29,
        },
        Triangle {
            a: 29,
            b: 32,
            c: 30,
        },
        Triangle {
            a: 30,
            b: 32,
            c: 31,
        },
        Triangle { a: 31, b: 32, c: 0 },
    ]
}
