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

use crate::test_data::{
    PrincipalDeformation, assert_relative_close, assert_relative_norm_close,
    test_general_deformations, test_pure_compressions, test_pure_expansions,
};

use super::*;

const RELATIVE_TOLERANCE: T = 1e-4;

fn sheared_deformations() -> Vec<PrincipalDeformation> {
    test_general_deformations()
}

fn pure_volume_changes() -> Vec<PrincipalDeformation> {
    [test_pure_compressions(), test_pure_expansions()].concat()
}

#[test]
fn shear_is_removed_while_volume_and_rotation_stay() {
    for deformation in sheared_deformations() {
        let position_gradient = deformation.position_gradient();
        let projected = fluid_plasticity(&position_gradient);
        let volume_ratio = position_gradient.determinant();

        assert_relative_close(volume_ratio, projected.determinant(), RELATIVE_TOLERANCE);
        assert_relative_norm_close(
            &Matrix3::from_diagonal_element(volume_ratio.powf(2. / 3.)),
            &(projected.transpose() * projected),
            RELATIVE_TOLERANCE,
        );
        assert_relative_norm_close(
            &deformation.rotation(),
            &PrincipalDeformation::from_position_gradient(&projected).rotation(),
            RELATIVE_TOLERANCE,
        );
    }
}

#[test]
fn pure_volume_change_is_unchanged() {
    for deformation in pure_volume_changes() {
        let position_gradient = deformation.position_gradient();
        assert_relative_norm_close(
            &position_gradient,
            &fluid_plasticity(&position_gradient),
            RELATIVE_TOLERANCE,
        );
    }
}

#[test]
fn projection_is_idempotent() {
    for deformation in sheared_deformations() {
        let once = fluid_plasticity(&deformation.position_gradient());
        let twice = fluid_plasticity(&once);
        assert_relative_norm_close(&once, &twice, RELATIVE_TOLERANCE);
    }
}
