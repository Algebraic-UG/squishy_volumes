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

use nalgebra::{Matrix3, Rotation3, Vector3};

use crate::T;

/// A position gradient `F = left · diag(stretches) · rightᵀ` that keeps its factors,
/// so expectations can be derived in principal space.
#[derive(Clone, Copy, Debug)]
pub struct PrincipalDeformation {
    pub left: Matrix3<T>,
    pub stretches: Vector3<T>,
    pub right: Matrix3<T>,
}

impl PrincipalDeformation {
    pub fn position_gradient(&self) -> Matrix3<T> {
        self.left * Matrix3::from_diagonal(&self.stretches) * self.right.transpose()
    }

    /// The rotational part `left · rightᵀ` of the polar decomposition.
    pub fn rotation(&self) -> Matrix3<T> {
        self.left * self.right.transpose()
    }

    /// The logarithmic principal strain `ln(stretches)`.
    pub fn hencky_strain(&self) -> Vector3<T> {
        self.stretches.map(T::ln)
    }

    pub fn with_stretches(self, stretches: Vector3<T>) -> Self {
        Self { stretches, ..self }
    }
}

/// The identity and a few generic rotations.
pub fn test_rotations() -> impl Iterator<Item = Matrix3<T>> + Clone {
    [
        Rotation3::identity(),
        Rotation3::from_euler_angles(0.3, -1.1, 2.4),
        Rotation3::from_axis_angle(&Vector3::y_axis(), 1.),
        Rotation3::from_euler_angles(-2.9, 0.7, -0.2),
    ]
    .into_iter()
    .map(|rotation| *rotation.matrix())
}

/// Every stretch combined with every pair of [`test_rotations`] as left and right factor.
/// The identity pair yields exactly diagonal position gradients.
pub fn test_rotated_deformations(
    stretches: impl IntoIterator<Item = Vector3<T>> + Clone,
) -> Vec<PrincipalDeformation> {
    test_rotations()
        .flat_map(|left| test_rotations().map(move |right| (left, right)))
        .flat_map(|(left, right)| {
            stretches
                .clone()
                .into_iter()
                .map(move |stretches| PrincipalDeformation {
                    left,
                    stretches,
                    right,
                })
        })
        .collect()
}

/// Uniform stretches `s·(1, 1, 1)` with `s < 1`, so the deviatoric strain vanishes.
pub fn test_pure_compressions() -> Vec<PrincipalDeformation> {
    test_rotated_deformations([0.5, 0.8, 0.95, 0.999].map(Vector3::repeat))
}

/// Stretches `s·(1 + δ, 1, 1 − δ)` with `s < 1` and `δ = 10⁻³`,
/// so the deviatoric strain is tiny but nonzero.
pub fn test_nearly_pure_compressions() -> Vec<PrincipalDeformation> {
    let shear = 1e-3;
    test_rotated_deformations(
        [0.5, 0.8, 0.95].map(|compression| Vector3::new(1. + shear, 1., 1. - shear) * compression),
    )
}

/// Uniform stretches `s·(1, 1, 1)` with `s > 1`.
pub fn test_pure_expansions() -> Vec<PrincipalDeformation> {
    test_rotated_deformations([1.001, 1.2, 2.].map(Vector3::repeat))
}

/// Anisotropic stretches, some with a repeated pair, combined with every pair of
/// [`test_rotations`]. All are well away from inversion.
pub fn test_general_deformations() -> Vec<PrincipalDeformation> {
    test_rotated_deformations([
        Vector3::new(1.3, 0.9, 0.7),
        Vector3::new(0.6, 1.1, 1.8),
        Vector3::new(1.02, 1., 0.97),
        Vector3::new(0.4, 0.5, 2.5),
        Vector3::new(1.5, 1.5, 0.8),
        Vector3::new(0.7, 1.2, 1.2),
    ])
}
