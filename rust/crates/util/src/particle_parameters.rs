// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use itertools::izip;

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

pub struct SplitParameters {
    pub density: Vec<T>,
    pub initial_volume: Vec<T>,
    pub viscosity_dynamic: Vec<T>,
    pub viscosity_bulk: Vec<T>,
    pub youngs_modulus: Vec<T>,
    pub poissons_ratio: Vec<T>,
    pub sand_alpha: Vec<T>,
    pub bulk_modulus: Vec<T>,

    // TODO: once blender can read ints we can change this
    pub exponent: Vec<T>,
}

impl From<&[ParticleParameters]> for SplitParameters {
    fn from(value: &[ParticleParameters]) -> Self {
        let mut result = Self {
            density: Vec::with_capacity(value.len()),
            initial_volume: Vec::with_capacity(value.len()),
            viscosity_dynamic: Vec::with_capacity(value.len()),
            viscosity_bulk: Vec::with_capacity(value.len()),
            youngs_modulus: Vec::with_capacity(value.len()),
            poissons_ratio: Vec::with_capacity(value.len()),
            sand_alpha: Vec::with_capacity(value.len()),
            bulk_modulus: Vec::with_capacity(value.len()),
            exponent: Vec::with_capacity(value.len()),
        };
        for ParticleParameters {
            density,
            initial_volume,
            viscosity_dynamic,
            viscosity_bulk,
            youngs_modulus,
            poissons_ratio,
            sand_alpha,
            bulk_modulus,
            exponent,
        } in value
        {
            result.density.push(*density);
            result.initial_volume.push(*initial_volume);
            result.viscosity_dynamic.push(*viscosity_dynamic);
            result.viscosity_bulk.push(*viscosity_bulk);
            result.youngs_modulus.push(*youngs_modulus);
            result.poissons_ratio.push(*poissons_ratio);
            result.sand_alpha.push(*sand_alpha);
            result.bulk_modulus.push(*bulk_modulus);
            result.exponent.push(*exponent as T);
        }
        result
    }
}

impl SplitParameters {
    pub fn to_zipped(self) -> Vec<ParticleParameters> {
        izip!(
            self.density,
            self.initial_volume,
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
                initial_volume,
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
                    initial_volume,
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
        .collect()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            density: Vec::with_capacity(capacity),
            initial_volume: Vec::with_capacity(capacity),
            viscosity_dynamic: Vec::with_capacity(capacity),
            viscosity_bulk: Vec::with_capacity(capacity),
            youngs_modulus: Vec::with_capacity(capacity),
            poissons_ratio: Vec::with_capacity(capacity),
            sand_alpha: Vec::with_capacity(capacity),
            bulk_modulus: Vec::with_capacity(capacity),
            exponent: Vec::with_capacity(capacity),
        }
    }
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
