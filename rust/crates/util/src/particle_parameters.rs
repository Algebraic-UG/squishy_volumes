// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use itertools::izip;

use crate::{
    T,
    attribute::{self, Trait as _},
};

#[repr(C)]
#[derive(
    Clone, Copy, bytemuck::Zeroable, Debug, PartialEq, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(not(use_f64), derive(bytemuck::Pod))]
pub struct ParticleParameters {
    pub density: T,
    pub initial_size: T,
    pub viscosity_dynamic: T,
    pub viscosity_bulk: T,
    pub youngs_modulus: T,
    pub poissons_ratio: T,
    pub sand_alpha: T,
    pub bulk_modulus: T,
    pub exponent: i32,
}

pub struct SplitParameters {
    pub density: Vec<T>,
    pub initial_size: Vec<T>,
    pub viscosity_dynamic: Vec<T>,
    pub viscosity_bulk: Vec<T>,
    pub youngs_modulus: Vec<T>,
    pub poissons_ratio: Vec<T>,
    pub sand_alpha: Vec<T>,
    pub bulk_modulus: Vec<T>,

    // TODO: once blender can read ints we can change this
    pub exponent: Vec<T>,
}

impl SplitParameters {
    pub fn to_zipped(self) -> impl Iterator<Item = ParticleParameters> {
        izip!(
            self.density,
            self.initial_size,
            self.viscosity_dynamic,
            self.viscosity_bulk,
            self.youngs_modulus,
            self.poissons_ratio,
            self.sand_alpha,
            self.bulk_modulus,
            self.exponent,
        )
        .map(
            |(
                density,
                initial_size,
                viscosity_dynamic,
                viscosity_bulk,
                youngs_modulus,
                poissons_ratio,
                sand_alpha,
                bulk_modulus,
                exponent,
            )| {
                ParticleParameters {
                    density,
                    initial_size,
                    viscosity_dynamic,
                    viscosity_bulk,
                    youngs_modulus,
                    poissons_ratio,
                    sand_alpha,
                    bulk_modulus,
                    exponent: exponent as i32,
                }
            },
        )
    }
}

impl Default for ParticleParameters {
    fn default() -> Self {
        Self {
            density: attribute::Density::DEFAULT as T,
            initial_size: attribute::InitialSize::DEFAULT as T,
            viscosity_dynamic: attribute::ViscosityDynamic::DEFAULT as T,
            viscosity_bulk: attribute::ViscosityBulk::DEFAULT as T,
            youngs_modulus: attribute::YoungsModulus::DEFAULT as T,
            poissons_ratio: attribute::PoissonsRatio::DEFAULT as T,
            sand_alpha: attribute::SandAlpha::DEFAULT as T,
            bulk_modulus: attribute::BulkModulus::DEFAULT as T,
            exponent: attribute::Exponent::DEFAULT as i32,
        }
    }
}

impl ParticleParameters {
    #[inline]
    pub fn mass(&self) -> T {
        self.density * self.initial_volume()
    }

    #[inline]
    pub fn initial_volume(&self) -> T {
        self.initial_size * self.initial_size * self.initial_size
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
