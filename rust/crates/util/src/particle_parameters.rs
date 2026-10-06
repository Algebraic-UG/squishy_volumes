// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use crate::T;

#[repr(C)]
#[derive(
    Clone, Copy, bytemuck::Zeroable, Debug, PartialEq, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(not(use_f64), derive(bytemuck::Pod))]
pub struct ParticleParameters {
    pub density: T,
    pub initial_volume: T,
    pub viscosity_dynamic: T,
    pub viscosity_bulk: T,
    pub youngs_modulus: T,
    pub poissons_ratio: T,
    pub sand_alpha: T,
    pub bulk_modulus: T,
    pub exponent: i32,
}

impl Default for ParticleParameters {
    fn default() -> Self {
        Self {
            density: 1000.,
            initial_volume: 0.01,
            viscosity_dynamic: 3.,
            viscosity_bulk: 3.,
            youngs_modulus: 10000.,
            poissons_ratio: 0.3,
            sand_alpha: 0.3,
            bulk_modulus: 10000.,
            exponent: 2,
        }
    }
}

impl ParticleParameters {
    #[inline]
    pub fn mass(&self) -> T {
        self.density * self.initial_volume
    }

    // Wikipedia: Lamé parameters (this is the "second")
    #[inline]
    pub fn mu(&self) -> T {
        self.youngs_modulus / 2. / (1. + self.poissons_ratio)
    }

    // Wikipedia: Lamé parameters (this is the "first")
    #[inline]
    pub fn lambda(&self) -> T {
        self.youngs_modulus * self.poissons_ratio
            / (1. + self.poissons_ratio)
            / (1. - 2. * self.poissons_ratio)
    }
}
