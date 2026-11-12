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

use nalgebra::{Matrix3, Vector3};

use crate::{ParticleParameters, T};

use super::{
    PrincipalDeformation, test_nearly_pure_compressions, test_pure_compressions,
    test_pure_expansions, test_shear_stiff_parameters,
};

/// A position gradient before sand plasticity and the result expected from the
/// Drucker–Prager model, derived independently of the implementation.
#[derive(Clone, Copy, Debug)]
pub struct SandPlasticityCase {
    pub parameters: ParticleParameters,
    pub position_gradient: Matrix3<T>,
    pub expected: Matrix3<T>,
}

fn cases_for_all_parameters(
    parameters: impl Iterator<Item = ParticleParameters>,
    deformations: &[PrincipalDeformation],
    expected: impl Fn(&ParticleParameters, &PrincipalDeformation) -> Matrix3<T>,
) -> Vec<SandPlasticityCase> {
    parameters
        .flat_map(|parameters| {
            deformations
                .iter()
                .map(move |deformation| (parameters, deformation))
        })
        .map(|(parameters, deformation)| SandPlasticityCase {
            parameters,
            position_gradient: deformation.position_gradient(),
            expected: expected(&parameters, deformation),
        })
        .collect()
}

/// The Drucker–Prager plastic multiplier `‖ε̂‖ + (3λ + 2μ) / (2μ) · tr ε · α`,
/// positive outside the cone, zero on its surface and negative inside.
pub fn sand_plastic_multiplier(
    parameters: &ParticleParameters,
    deformation: &PrincipalDeformation,
) -> T {
    let strain = deformation.hencky_strain();
    let volumetric_strain = strain.sum();
    let deviatoric_strain = strain - Vector3::repeat(volumetric_strain / 3.);
    let (mu, lambda) = (parameters.mu(), parameters.lambda());
    deviatoric_strain.norm()
        + (3. * lambda + 2. * mu) / (2. * mu) * volumetric_strain * parameters.sand_alpha
}

fn unchanged_inside_cone(
    parameters: &ParticleParameters,
    deformation: &PrincipalDeformation,
) -> Matrix3<T> {
    assert!(
        sand_plastic_multiplier(parameters, deformation) <= 0.,
        "fixture {deformation:?} is not inside the cone for {parameters:?}",
    );
    deformation.position_gradient()
}

/// Pure compression lies on the cone axis, so the position gradient stays unchanged.
pub fn sand_pure_compression_cases() -> Vec<SandPlasticityCase> {
    cases_for_all_parameters(
        test_shear_stiff_parameters(),
        &test_pure_compressions(),
        |_, deformation| deformation.position_gradient(),
    )
}

/// A tiny shear on top of a compression stays inside the cone, so the position
/// gradient stays unchanged. Building the cases asserts `Δγ ≤ 0` in closed form.
pub fn sand_nearly_pure_compression_cases() -> Vec<SandPlasticityCase> {
    cases_for_all_parameters(
        test_shear_stiff_parameters(),
        &test_nearly_pure_compressions(),
        unchanged_inside_cone,
    )
}

/// With `α = 0` the cone degenerates to its axis, so the shear is removed while
/// the volume is kept: the stretches become `exp(tr ε / 3)`.
pub fn sand_frictionless_compression_cases() -> Vec<SandPlasticityCase> {
    let frictionless_parameters =
        test_shear_stiff_parameters().map(|parameters| ParticleParameters {
            sand_alpha: 0.,
            ..parameters
        });
    cases_for_all_parameters(
        frictionless_parameters,
        &test_nearly_pure_compressions(),
        |_, deformation| {
            let volume_preserving_stretch = (deformation.hencky_strain().sum() / 3.).exp();
            deformation
                .with_stretches(Vector3::repeat(volume_preserving_stretch))
                .position_gradient()
        },
    )
}

/// Expansion projects to the cone tip, leaving only the rotation.
pub fn sand_expansion_cases() -> Vec<SandPlasticityCase> {
    cases_for_all_parameters(
        test_shear_stiff_parameters(),
        &test_pure_expansions(),
        |_, deformation| deformation.rotation(),
    )
}
