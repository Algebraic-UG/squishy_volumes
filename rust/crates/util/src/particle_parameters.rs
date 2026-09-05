// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use crate::T;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ViscosityParameters {
    pub dynamic: T,
    pub bulk: T,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, Default)]
pub struct ParticleParameters {
    pub mass: T,
    pub initial_volume: T,
    pub viscosity: Option<ViscosityParameters>,
    pub specific: SpecificParticleParameters,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum SpecificParticleParameters {
    Solid {
        mu: T,
        lambda: T,
        sand_alpha: Option<T>,
    },
    Fluid {
        exponent: i32,
        bulk_modulus: T,
    },
}

impl Default for SpecificParticleParameters {
    fn default() -> Self {
        Self::Solid {
            mu: 0.,
            lambda: 0.,
            sand_alpha: None,
        }
    }
}
