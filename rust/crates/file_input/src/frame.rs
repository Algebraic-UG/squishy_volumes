// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub struct ParticlesInput {
    pub flags: Vec<i32>,
    pub transforms: Vec<f32>,
    pub sizes: Vec<f32>,
    pub densities: Vec<f32>,
    pub youngs_moduluses: Vec<f32>,
    pub poissons_ratios: Vec<f32>,
    pub initial_positions: Vec<f32>,
    pub initial_velocities: Vec<f32>,
    pub viscosities_dynamic: Vec<f32>,
    pub viscosities_bulk: Vec<f32>,
    pub exponents: Vec<i32>,
    pub bulk_moduluses: Vec<f32>,
    pub sand_alphas: Vec<f32>,
    pub goal_positions: Vec<f32>,
}

#[cfg(test)]
impl ParticlesInput {
    fn random(n: usize, rng: &mut impl rand::Rng) -> Self {
        use rand::RngExt as _;
        Self {
            flags: rng.random_iter().take(n).collect(),
            transforms: rng.random_iter().take(n * 16).collect(),
            sizes: rng.random_iter().take(n).collect(),
            densities: rng.random_iter().take(n).collect(),
            youngs_moduluses: rng.random_iter().take(n).collect(),
            poissons_ratios: rng.random_iter().take(n).collect(),
            initial_positions: rng.random_iter().take(n * 3).collect(),
            initial_velocities: rng.random_iter().take(n * 3).collect(),
            viscosities_dynamic: rng.random_iter().take(n).collect(),
            viscosities_bulk: rng.random_iter().take(n).collect(),
            exponents: rng.random_iter().take(n).collect(),
            bulk_moduluses: rng.random_iter().take(n).collect(),
            sand_alphas: rng.random_iter().take(n).collect(),
            goal_positions: rng.random_iter().take(n * 3).collect(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub struct ColliderInput {
    pub vertex_positions: Vec<f32>,
    pub triangles: Vec<i32>,
    pub triangle_frictions: Vec<f32>,
}

#[cfg(test)]
impl ColliderInput {
    fn random(n: usize, rng: &mut impl rand::Rng) -> Self {
        use rand::RngExt as _;
        Self {
            vertex_positions: rng.random_iter().take(n * 3).collect(),
            triangles: rng.random_iter().take(n * 3).collect(),
            triangle_frictions: rng.random_iter().take(n).collect(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct InputFrame {
    pub gravity: [f32; 3],
    pub particles_inputs: std::collections::BTreeMap<String, ParticlesInput>,
    pub collider_inputs: std::collections::BTreeMap<String, ColliderInput>,
}

#[cfg(test)]
impl InputFrame {
    pub fn test_input_0() -> Self {
        use rand::{SeedableRng, rngs::ChaCha8Rng};
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let n = 10;
        Self {
            gravity: [0.; 3],
            particles_inputs: [
                ("foo".to_string(), ParticlesInput::random(n, &mut rng)),
                ("bar".to_string(), ParticlesInput::random(n, &mut rng)),
            ]
            .into_iter()
            .collect(),
            collider_inputs: [("car".to_string(), ColliderInput::random(n, &mut rng))]
                .into_iter()
                .collect(),
        }
    }

    pub fn test_input_1() -> Self {
        use rand::{SeedableRng, rngs::ChaCha8Rng};
        let mut rng = ChaCha8Rng::seed_from_u64(69);
        let n = 10;
        Self {
            gravity: [0., 0., -10.],
            particles_inputs: [
                ("foo".to_string(), ParticlesInput::random(n, &mut rng)),
                ("bar".to_string(), ParticlesInput::random(n, &mut rng)),
                ("car".to_string(), ParticlesInput::random(n, &mut rng)),
            ]
            .into_iter()
            .collect(),
            collider_inputs: Default::default(),
        }
    }

    pub fn test_input_2() -> Self {
        Self {
            gravity: [0., 0., 10.],
            particles_inputs: Default::default(),
            collider_inputs: Default::default(),
        }
    }
}
