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

use crate::{ParticleParameters, T, test_data::assert_relative_close};

const RELATIVE_TOLERANCE: T = 1e-12;

fn mu_lambda(youngs_modulus: T, poissons_ratio: T) -> (T, T) {
    let parameters = ParticleParameters {
        youngs_modulus,
        poissons_ratio,
        ..Default::default()
    };
    (parameters.mu(), parameters.lambda())
}

#[test]
fn known_values() {
    let (mu, lambda) = mu_lambda(1., 0.25);
    assert_relative_close(0.4, mu, RELATIVE_TOLERANCE);
    assert_relative_close(0.4, lambda, RELATIVE_TOLERANCE);

    let (mu, lambda) = mu_lambda(2.6, 0.3);
    assert_relative_close(1., mu, RELATIVE_TOLERANCE);
    assert_relative_close(1.5, lambda, RELATIVE_TOLERANCE);

    let (mu, lambda) = mu_lambda(3., 0.);
    assert_relative_close(1.5, mu, RELATIVE_TOLERANCE);
    assert_eq!(lambda, 0.);
}

#[test]
fn round_trip_to_youngs_modulus_and_poissons_ratio() {
    for youngs_modulus in [1., 1e3, 1e6] {
        for poissons_ratio in [0.1, 0.3, 0.45, 0.499] {
            let (mu, lambda) = mu_lambda(youngs_modulus, poissons_ratio);
            assert_relative_close(
                youngs_modulus,
                mu * (3. * lambda + 2. * mu) / (lambda + mu),
                1e-9,
            );
            assert_relative_close(poissons_ratio, lambda / (2. * (lambda + mu)), 1e-9);
        }
    }
}
