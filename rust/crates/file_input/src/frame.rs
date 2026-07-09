// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub struct ParticlesInput {
    pub flags: Vec<u32>,
    pub transforms: Vec<[[f32; 4]; 4]>,
    pub sizes: Vec<f32>,
    pub densities: Vec<f32>,
    pub youngs_moduluses: Vec<f32>,
    pub poissons_ratios: Vec<f32>,
    pub initial_positions: Vec<[f32; 3]>,
    pub initial_velocities: Vec<[f32; 3]>,
    pub viscosities_dynamic: Vec<f32>,
    pub viscosities_bulk: Vec<f32>,
    pub exponents: Vec<u32>,
    pub bulk_moduluses: Vec<f32>,
    pub sand_alphas: Vec<f32>,
    pub goal_positions: Vec<[f32; 3]>,
}

#[cfg(test)]
impl ParticlesInput {
    fn random(n: usize, rng: &mut impl rand::Rng) -> Self {
        use rand::RngExt as _;
        Self {
            flags: rng.random_iter().take(n).collect(),
            transforms: rng.random_iter().take(n).collect(),
            sizes: rng.random_iter().take(n).collect(),
            densities: rng.random_iter().take(n).collect(),
            youngs_moduluses: rng.random_iter().take(n).collect(),
            poissons_ratios: rng.random_iter().take(n).collect(),
            initial_positions: rng.random_iter().take(n).collect(),
            initial_velocities: rng.random_iter().take(n).collect(),
            viscosities_dynamic: rng.random_iter().take(n).collect(),
            viscosities_bulk: rng.random_iter().take(n).collect(),
            exponents: rng.random_iter().take(n).collect(),
            bulk_moduluses: rng.random_iter().take(n).collect(),
            sand_alphas: rng.random_iter().take(n).collect(),
            goal_positions: rng.random_iter().take(n).collect(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub struct ColliderInput {
    pub vertex_positions: Vec<[f32; 3]>,
    pub triangle_indices: Vec<[u32; 3]>,
    pub triangle_frictions: Vec<f32>,
}

#[cfg(test)]
impl ColliderInput {
    fn random(num_vertices: usize, num_triangles: usize, rng: &mut impl rand::Rng) -> Self {
        use rand::RngExt as _;
        Self {
            vertex_positions: rng.random_iter().take(num_vertices).collect(),
            triangle_indices: rng.random_iter().take(num_triangles).collect(),
            triangle_frictions: rng.random_iter().take(num_triangles).collect(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct InputFrame {
    pub gravity: [f32; 3],
    pub particles_inputs: std::collections::BTreeMap<String, ParticlesInput>,
    pub collider_inputs: std::collections::BTreeMap<String, ColliderInput>,
}

macro_rules! check_length {
    ($name:expr, $expected:expr, $field:expr) => {
        if $field.len() != $expected {
            Err(crate::FrameVerifcationError::LengthMismatch {
                name: $name.clone(),
                attribute: stringify!($field),
                found: $field.len(),
                expected: $expected,
            })
        } else {
            Ok(())
        }
    };
}

impl InputFrame {
    pub fn verify(&self, header: &crate::InputHeader) -> Result<(), crate::FrameVerifcationError> {
        for name in self.particles_inputs.keys() {
            if !matches!(
                header.objects.get(name).ok_or(
                    crate::FrameVerifcationError::ObjectNotInHeader { name: name.clone() }
                )?,
                crate::InputObject::Particles { .. }
            ) {
                return Err(crate::FrameVerifcationError::ObjectChangedType { name: name.clone() });
            }
        }
        for name in self.collider_inputs.keys() {
            if !matches!(
                header.objects.get(name).ok_or(
                    crate::FrameVerifcationError::ObjectNotInHeader { name: name.clone() }
                )?,
                crate::InputObject::Collider { .. }
            ) {
                return Err(crate::FrameVerifcationError::ObjectChangedType { name: name.clone() });
            }
        }

        for (name, object) in header.objects.iter() {
            match object {
                crate::InputObject::Particles { num_particles } => {
                    let Some(ParticlesInput {
                        flags,
                        transforms,
                        sizes,
                        densities,
                        youngs_moduluses,
                        poissons_ratios,
                        initial_positions,
                        initial_velocities,
                        viscosities_dynamic,
                        viscosities_bulk,
                        exponents,
                        bulk_moduluses,
                        sand_alphas,
                        goal_positions,
                    }) = self.particles_inputs.get(name)
                    else {
                        continue;
                    };
                    check_length!(name, *num_particles, flags)?;
                    check_length!(name, *num_particles, transforms)?;
                    check_length!(name, *num_particles, sizes)?;
                    check_length!(name, *num_particles, densities)?;
                    check_length!(name, *num_particles, youngs_moduluses)?;
                    check_length!(name, *num_particles, poissons_ratios)?;
                    check_length!(name, *num_particles, initial_positions)?;
                    check_length!(name, *num_particles, initial_velocities)?;
                    check_length!(name, *num_particles, viscosities_bulk)?;
                    check_length!(name, *num_particles, viscosities_dynamic)?;
                    check_length!(name, *num_particles, exponents)?;
                    check_length!(name, *num_particles, bulk_moduluses)?;
                    check_length!(name, *num_particles, sand_alphas)?;
                    check_length!(name, *num_particles, goal_positions)?;
                }
                crate::InputObject::Collider {
                    num_vertices,
                    num_triangles,
                } => {
                    let ColliderInput {
                        vertex_positions,
                        triangle_indices,
                        triangle_frictions,
                    } = self.collider_inputs.get(name).ok_or(
                        crate::FrameVerifcationError::ColliderInputMissing(name.clone()),
                    )?;
                    check_length!(name, *num_vertices, vertex_positions)?;
                    check_length!(name, *num_triangles, triangle_indices)?;
                    check_length!(name, *num_triangles, triangle_frictions)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
impl InputFrame {
    pub fn test_input_0(num_particles: usize, num_vertices: usize, num_triangles: usize) -> Self {
        use rand::{SeedableRng, rngs::ChaCha8Rng};
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        Self {
            gravity: [0.; 3],
            particles_inputs: [
                (
                    "foo".to_string(),
                    ParticlesInput::random(num_particles, &mut rng),
                ),
                (
                    "bar".to_string(),
                    ParticlesInput::random(num_particles, &mut rng),
                ),
            ]
            .into_iter()
            .collect(),
            collider_inputs: [(
                "car".to_string(),
                ColliderInput::random(num_vertices, num_triangles, &mut rng),
            )]
            .into_iter()
            .collect(),
        }
    }

    pub fn test_input_1(num_particles: usize) -> Self {
        use rand::{SeedableRng, rngs::ChaCha8Rng};
        let mut rng = ChaCha8Rng::seed_from_u64(69);
        Self {
            gravity: [0., 0., -10.],
            particles_inputs: [
                (
                    "foo".to_string(),
                    ParticlesInput::random(num_particles, &mut rng),
                ),
                (
                    "bar".to_string(),
                    ParticlesInput::random(num_particles, &mut rng),
                ),
                (
                    "car".to_string(),
                    ParticlesInput::random(num_particles, &mut rng),
                ),
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
