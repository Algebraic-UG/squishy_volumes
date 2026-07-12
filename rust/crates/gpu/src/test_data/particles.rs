// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix1x3, Matrix3, Matrix4x3, Vector3, Vector4, stack};
use rand::{RngExt, SeedableRng as _, rngs::ChaCha8Rng};
use squishy_volumes_file_frame::{ParticleFlags, ParticleParameters, SpecificParticleParameters};
use squishy_volumes_util::Aabb;

use crate::PositionAndColliderBits;

pub struct TestParticles {
    pub particle_flags: Vec<ParticleFlags>,
    pub particle_parameters: Vec<ParticleParameters>,
    pub particle_goals_start: Vec<Vector4<f32>>,
    pub particle_goals_end: Vec<Vector4<f32>>,
    pub particle_positions_and_collider_bits: Vec<PositionAndColliderBits>,
    pub particle_position_gradients: Vec<Matrix4x3<f32>>,
    pub particle_velocities: Vec<Vector4<f32>>,
    pub particle_velocity_gradients: Vec<Matrix4x3<f32>>,
}

pub enum ParticleSampling {
    Random,
    Neat(f32),
}

pub fn test_position_gradients_random(n: usize) -> Vec<Matrix3<f32>> {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let mut tmp = Vec::new();

    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let mut position_gradient;
    for _ in 0..n {
        loop {
            position_gradient = Matrix3::from_fn(|_, _| rng.random::<f32>());
            let d = position_gradient.determinant().abs();
            if d > 1e-1 && d < 1e+1 {
                break;
            }
        }

        if position_gradient.determinant() < 0. {
            position_gradient *= -1.;
        }
        tmp.push(position_gradient);
    }

    tmp
}

pub fn test_lame_parameters<T: rand::Rng>(
    rng: &mut T,
) -> impl Iterator<Item = ParticleParameters> + use<'_, T> {
    squishy_volumes_util::test_lame_parameters().map(|[mu, lambda]| ParticleParameters {
        mass: rng.random_range(0.1..1.0),
        initial_volume: rng.random_range(0.1..1.0),
        viscosity: None,
        specific: SpecificParticleParameters::Solid {
            mu,
            lambda,
            sand_alpha: None,
        },
    })
}

pub fn test_inviscid_parameters(
    rng: &mut impl rand::Rng,
) -> impl Iterator<Item = ParticleParameters> {
    squishy_volumes_util::test_inviscid_parameters().map(|(bulk_modulus, exponent)| {
        ParticleParameters {
            mass: rng.random_range(0.1..1.0),
            initial_volume: rng.random_range(0.1..1.0),
            viscosity: None,
            specific: SpecificParticleParameters::Fluid {
                exponent,
                bulk_modulus,
            },
        }
    })
}

impl TestParticles {
    pub fn new(num_particles: usize, aabb: Aabb<Vector3<f32>>, sampling: ParticleSampling) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(33);
        let particle_parameters = test_lame_parameters(&mut rng)
            .collect::<Vec<_>>()
            .into_iter()
            .chain(test_inviscid_parameters(&mut rng))
            .collect::<Vec<_>>()
            .into_iter()
            .cycle()
            .take(num_particles)
            .collect::<Vec<_>>();

        let particle_flags = particle_parameters
            .iter()
            .map(|p| match p.specific {
                SpecificParticleParameters::Solid { .. } => ParticleFlags::IS_SOLID,
                SpecificParticleParameters::Fluid { .. } => ParticleFlags::IS_FLUID,
            })
            .collect::<Vec<_>>();
        let particle_goals_start = vec![Vector4::zeros(); num_particles];
        let particle_goals_end = vec![Vector4::zeros(); num_particles];
        let particle_positions_and_collider_bits = match sampling {
            ParticleSampling::Random => (0..num_particles)
                .map(|_| PositionAndColliderBits {
                    position: Vector3::new(
                        rng.random_range(-10.0..10.),
                        rng.random_range(-10.0..10.),
                        rng.random_range(-10.0..10.),
                    ),
                    collider_bits: rng.random(),
                })
                .collect(),
            ParticleSampling::Neat(spacing) => {
                let (count, it) = aabb.lattice(spacing);
                assert!(count > num_particles);
                it.take(num_particles)
                    .map(|position| PositionAndColliderBits {
                        position,
                        collider_bits: rng.random(),
                    })
                    .collect()
            }
        };

        let particle_position_gradients = test_position_gradients_random(num_particles)
            .into_iter()
            .map(|m| stack![m; Matrix1x3::zeros()])
            .collect::<Vec<_>>();

        let particle_velocities = (0..num_particles)
            .map(|_| {
                Vector4::new(
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    0.,
                )
            })
            .collect::<Vec<_>>();
        let particle_velocity_gradients = (0..num_particles)
            .map(|_| {
                stack![
                    Matrix3::new(
                        rng.random_range(-1.0..1.),
                        rng.random_range(-1.0..1.),
                        rng.random_range(-1.0..1.),
                        rng.random_range(-1.0..1.),
                        rng.random_range(-1.0..1.),
                        rng.random_range(-1.0..1.),
                        rng.random_range(-1.0..1.),
                        rng.random_range(-1.0..1.),
                        rng.random_range(-1.0..1.),
                    );
                    Matrix1x3::zeros()
                ]
            })
            .collect::<Vec<_>>();

        Self {
            particle_flags,
            particle_parameters,
            particle_goals_start,
            particle_goals_end,
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        }
    }
}
