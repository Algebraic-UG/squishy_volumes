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

fn trace(m: mat3x3f) -> f32 {
    return m[0].x + m[1].y + m[2].z;
}

fn from_diagonal(d: vec3f) -> mat3x3f {
    return mat3x3f(
        vec3f(d.x, 0., 0.),
        vec3f(0., d.y, 0.),
        vec3f(0., 0., d.z),
    );
}

fn from_diagonal_element(e: f32) -> mat3x3f {
    return mat3x3f(
        vec3f(e, 0., 0.),
        vec3f(0., e, 0.),
        vec3f(0., 0., e),
    );
}

fn outer(a: vec3f, b: vec3f) -> mat3x3f {
    return mat3x3f(
        a * b.x,
        a * b.y,
        a * b.z,
    );
}

fn mat3x3f_abs(m: mat3x3f) -> mat3x3f {
    return mat3x3f(abs(m[0]), abs(m[1]), abs(m[2]));
}

fn mat3x3f_max_elem(m: mat3x3f) -> f32 {
    let v = max(max(m[0], m[1]), m[2]);
    return max(max(v.x, v.y), v.z);
}
