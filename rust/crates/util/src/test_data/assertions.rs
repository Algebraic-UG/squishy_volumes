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

use nalgebra::SMatrix;

use crate::T;

/// Asserts `‖expected − actual‖ ≤ relative_tolerance · ‖expected‖` in the Frobenius norm.
pub fn assert_relative_norm_close<const R: usize, const C: usize>(
    expected: &SMatrix<T, R, C>,
    actual: &SMatrix<T, R, C>,
    relative_tolerance: T,
) {
    let error = (expected - actual).norm();
    let tolerance = relative_tolerance * expected.norm();
    assert!(
        error <= tolerance,
        "expected {expected} but got {actual}, error={error}, tolerance={tolerance}",
    );
}

/// Asserts `‖actual‖ ≤ relative_tolerance · scale` for a quantity expected to vanish,
/// where `scale` is the magnitude it would have without the cancellation under test.
pub fn assert_norm_negligible<const R: usize, const C: usize>(
    actual: &SMatrix<T, R, C>,
    scale: T,
    relative_tolerance: T,
) {
    let norm = actual.norm();
    let tolerance = relative_tolerance * scale;
    assert!(
        norm <= tolerance,
        "expected a negligible value but got {actual}, norm={norm}, tolerance={tolerance}",
    );
}

/// Asserts `|expected − actual| ≤ relative_tolerance · |expected|`.
pub fn assert_relative_close(expected: T, actual: T, relative_tolerance: T) {
    let error = (expected - actual).abs();
    let tolerance = relative_tolerance * expected.abs();
    assert!(
        error <= tolerance,
        "expected {expected} but got {actual}, error={error}, tolerance={tolerance}",
    );
}

/// Asserts `‖expected − actual‖ ≤ relative_tolerance · max(‖expected‖, scale)`,
/// where `scale` is the typical magnitude of the compared quantity, so vanishing
/// expectations are compared against that magnitude instead of zero.
pub fn assert_close_at_scale<const R: usize, const C: usize>(
    expected: &SMatrix<T, R, C>,
    actual: &SMatrix<T, R, C>,
    scale: T,
    relative_tolerance: T,
) {
    assert_norm_negligible(
        &(expected - actual),
        expected.norm().max(scale),
        relative_tolerance,
    );
}
