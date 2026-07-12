// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU64;

use crate::AllowedInBinding;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq, Default)]
pub struct ParticleParametersDevice {
    mass: f32,
    initial_volume: f32,
    viscosity_dynamic: f32,
    viscosity_bulk: f32,
    mu: f32,
    lambda: f32,
    sand_alpha: f32,
    bulk_modulus: f32,
    exponent: i32,
}

impl AllowedInBinding for squishy_volumes_file_frame::ParticleFlags {}
impl AllowedInBinding for ParticleParametersDevice {
    const ALIGNMENT: NonZeroU64 = u32::ALIGNMENT;
}

impl From<&squishy_volumes_file_frame::ParticleParameters> for ParticleParametersDevice {
    fn from(
        squishy_volumes_file_frame::ParticleParameters {
            mass,
            initial_volume,
            viscosity,
            specific,
        }: &squishy_volumes_file_frame::ParticleParameters,
    ) -> Self {
        match specific.clone() {
            squishy_volumes_file_frame::SpecificParticleParameters::Solid {
                mu,
                lambda,
                sand_alpha,
            } => Self {
                mass: *mass,
                initial_volume: *initial_volume,
                viscosity_dynamic: viscosity
                    .map(|viscosity| viscosity.dynamic)
                    .unwrap_or_default(),
                viscosity_bulk: viscosity
                    .map(|viscosity| viscosity.bulk)
                    .unwrap_or_default(),
                mu,
                lambda,
                sand_alpha: sand_alpha.unwrap_or_default(),
                ..Default::default()
            },
            squishy_volumes_file_frame::SpecificParticleParameters::Fluid {
                exponent,
                bulk_modulus,
            } => Self {
                mass: *mass,
                initial_volume: *initial_volume,
                viscosity_dynamic: viscosity
                    .map(|viscosity| viscosity.dynamic)
                    .unwrap_or_default(),
                viscosity_bulk: viscosity
                    .map(|viscosity| viscosity.bulk)
                    .unwrap_or_default(),
                bulk_modulus,
                exponent,
                ..Default::default()
            },
        }
    }
}
