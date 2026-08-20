/// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ViscosityParameters {
    pub dynamic: f32,
    pub bulk: f32,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, Default)]
pub struct ParticleParameters {
    pub mass: f32,
    pub initial_volume: f32,
    pub viscosity: Option<ViscosityParameters>,
    pub specific: SpecificParticleParameters,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum SpecificParticleParameters {
    Solid {
        mu: f32,
        lambda: f32,
        sand_alpha: Option<f32>,
    },
    Fluid {
        exponent: i32,
        bulk_modulus: f32,
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
