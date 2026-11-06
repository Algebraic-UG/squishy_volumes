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

#[cfg(test)]
mod test;

use nalgebra::{Matrix3, Vector3};

use crate::T;

/// Drucker–Prager return mapping in Hencky strain for sand,
/// following Klár et al. 2016, "Drucker-Prager Elastoplasticity for Sand Animation".
///
/// With `F = U Σ Vᵀ`, `ε = log Σ` and its deviatoric part `ε̂`:
/// - Expansion (`tr ε ≥ 0`) or a material without shear stiffness projects
///   to the cone tip, leaving only the rotation `U Vᵀ`.
/// - Pure compression (`ε̂ = 0`) lies on the cone axis, so it is returned unchanged.
/// - Otherwise the strain is projected back onto the cone if it lies outside.
pub fn sand_plasticity(
    mu: T,
    lambda: T,
    sand_alpha: T,
    position_gradient: &Matrix3<T>,
) -> Matrix3<T> {
    let mut svd = position_gradient.svd(true, true);
    let hencky_strain = svd.singular_values.map(T::ln);
    let volumetric_strain = hencky_strain.sum();

    let projects_to_cone_tip = mu <= 0. || volumetric_strain >= 0.;
    if projects_to_cone_tip {
        return svd.u.unwrap() * svd.v_t.unwrap();
    }

    let deviatoric_strain = hencky_strain - Vector3::repeat(volumetric_strain / 3.);
    let deviatoric_strain_norm = deviatoric_strain.norm();
    let is_pure_compression = deviatoric_strain_norm == 0.;
    if is_pure_compression {
        return *position_gradient;
    }

    let plastic_multiplier = deviatoric_strain_norm
        + (3. * lambda + 2. * mu) / (2. * mu) * volumetric_strain * sand_alpha;
    let is_inside_cone = plastic_multiplier <= 0.;
    if is_inside_cone {
        return *position_gradient;
    }

    let projected_strain =
        hencky_strain - plastic_multiplier / deviatoric_strain_norm * deviatoric_strain;
    svd.singular_values = projected_strain.map(T::exp);
    svd.recompose().unwrap()
}
