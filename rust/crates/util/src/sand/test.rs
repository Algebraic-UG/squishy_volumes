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
    SandPlasticityCase, assert_relative_norm_close, sand_expansion_cases,
    sand_frictionless_compression_cases, sand_nearly_pure_compression_cases,
    sand_pure_compression_cases,
};

use super::*;

const RELATIVE_TOLERANCE: T = 1e-4;

fn check(cases: &[SandPlasticityCase]) {
    for SandPlasticityCase {
        parameters,
        position_gradient,
        expected,
    } in cases
    {
        let actual = sand_plasticity(
            parameters.mu(),
            parameters.lambda(),
            parameters.sand_alpha,
            position_gradient,
        );
        assert_relative_norm_close(expected, &actual, RELATIVE_TOLERANCE);
    }
}

#[test]
fn pure_compression_is_unchanged() {
    check(&sand_pure_compression_cases());
}

#[test]
fn nearly_pure_compression_is_unchanged() {
    check(&sand_nearly_pure_compression_cases());
}

#[test]
fn frictionless_compression_loses_shear() {
    check(&sand_frictionless_compression_cases());
}

#[test]
fn expansion_becomes_rotation() {
    check(&sand_expansion_cases());
}
