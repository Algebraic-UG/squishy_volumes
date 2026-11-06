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

use crate::{ParticleParameters, T, test_lame_parameters};

/// Particle parameters for every pair from [`test_lame_parameters`],
/// with all other parameters at their defaults.
pub fn test_solid_parameters() -> impl Iterator<Item = ParticleParameters> + Clone {
    test_lame_parameters().map(|[youngs_modulus, poissons_ratio]| ParticleParameters {
        youngs_modulus,
        poissons_ratio,
        ..Default::default()
    })
}

/// [`test_solid_parameters`] restricted to a positive shear modulus.
pub fn test_shear_stiff_parameters() -> impl Iterator<Item = ParticleParameters> + Clone {
    test_solid_parameters().filter(|parameters| parameters.mu() > 0.)
}

/// The Lamé parameters `[μ, λ]` of [`test_solid_parameters`].
pub fn test_mu_lambda() -> impl Iterator<Item = [T; 2]> + Clone {
    test_solid_parameters().map(|parameters| [parameters.mu(), parameters.lambda()])
}
