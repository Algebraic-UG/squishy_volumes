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

import super::consts::NORMALIZATION_EPS;

struct Triangle {
    a: u32,
    b: u32,
    c: u32,
}

fn try_normalize(n: vec3f, alternative: vec3f) -> vec3f {
    let l2 = dot(n, n);
    if l2 < NORMALIZATION_EPS * NORMALIZATION_EPS {
        return alternative;
    } else {
        return n / sqrt(l2);
    }
}
