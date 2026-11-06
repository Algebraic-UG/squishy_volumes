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

use nalgebra::Matrix3;

use crate::T;

/// Removes all shear from `F = U Σ Vᵀ` and keeps only its volume change:
/// the stretches become `(det Σ)^(1/3)`, the rotation `U Vᵀ` stays.
pub fn fluid_plasticity(position_gradient: &Matrix3<T>) -> Matrix3<T> {
    let mut svd = position_gradient.svd(true, true);
    let volume_preserving_stretch = svd.singular_values.product().powf(1. / 3.);
    svd.singular_values.fill(volume_preserving_stretch);
    svd.recompose().unwrap()
}
