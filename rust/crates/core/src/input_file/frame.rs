// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::BTreeMap;

use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;

use crate::ParticleFlags;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct ParticlesInput {
    pub flags: Vec<ParticleFlags>,
    pub transforms: Vec<T>,
    pub sizes: Vec<T>,
    pub densities: Vec<T>,
    pub youngs_moduluses: Vec<T>,
    pub poissons_ratios: Vec<T>,
    pub initial_positions: Vec<T>,
    pub initial_velocities: Vec<T>,
    pub viscosities_dynamic: Vec<T>,
    pub viscosities_bulk: Vec<T>,
    pub exponents: Vec<i32>,
    pub bulk_moduluses: Vec<T>,
    pub sand_alphas: Vec<T>,
    pub goal_positions: Vec<T>,
}

#[cfg(test)]
impl ParticlesInput {
    pub fn test_input() -> Self {
        use nalgebra::Matrix4;

        use crate::math::flat::{Flat3 as _, Flat16 as _};

        let n = 10;
        Self {
            flags: vec![
                ParticleFlags::IsSolid
                    & ParticleFlags::UseViscosity
                    & ParticleFlags::UseSandAlpha;
                n
            ],
            transforms: Matrix4::identity().flat().repeat(n),
            sizes: vec![0.25; n],
            densities: vec![1000.; n],
            youngs_moduluses: vec![100000.; n],
            poissons_ratios: vec![0.3; n],
            initial_positions: Vector3::zeros().flat().repeat(n),
            initial_velocities: Vector3::zeros().flat().repeat(n),
            viscosities_dynamic: vec![1.; n],
            viscosities_bulk: vec![1.; n],
            exponents: vec![2; n],
            bulk_moduluses: vec![1000.; n],
            sand_alphas: vec![0.3; n],
            goal_positions: Vector3::zeros().flat().repeat(n),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct ColliderInput {
    pub vertex_positions: Vec<T>,
    pub triangles: Vec<i32>,
    pub triangle_frictions: Vec<T>,
}

#[cfg(test)]
impl ColliderInput {
    pub fn test_input() -> Self {
        Self {
            vertex_positions: vec![
                0., 0., 0., //
                1., 0., 0., //
                0., 1., 0., //
            ],
            triangles: vec![0, 1, 2],
            triangle_frictions: vec![0.3],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InputFrame {
    pub gravity: Vector3<T>,
    pub particles_inputs: BTreeMap<String, ParticlesInput>,
    pub collider_inputs: BTreeMap<String, ColliderInput>,
}

#[cfg(test)]
impl InputFrame {
    pub fn test_input_0() -> Self {
        Self {
            gravity: Vector3::new(0., 0., 0.),
            particles_inputs: [
                ("foo".to_string(), ParticlesInput::test_input()),
                ("bar".to_string(), ParticlesInput::test_input()),
            ]
            .into_iter()
            .collect(),
            collider_inputs: [("car".to_string(), ColliderInput::test_input())]
                .into_iter()
                .collect(),
        }
    }

    pub fn test_input_1() -> Self {
        Self {
            gravity: Vector3::new(0., 0., -10.),
            particles_inputs: [
                ("foo".to_string(), ParticlesInput::test_input()),
                ("bar".to_string(), ParticlesInput::test_input()),
                ("car".to_string(), ParticlesInput::test_input()),
            ]
            .into_iter()
            .collect(),
            collider_inputs: Default::default(),
        }
    }

    pub fn test_input_2() -> Self {
        Self {
            gravity: Vector3::new(0., 0., 10.),
            particles_inputs: Default::default(),
            collider_inputs: Default::default(),
        }
    }
}
