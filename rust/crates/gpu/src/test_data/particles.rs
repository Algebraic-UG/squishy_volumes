use nalgebra::{Matrix1x3, Matrix3, Matrix4x3, Vector3, Vector4, stack};
use rand::{RngExt, SeedableRng as _, rngs::ChaCha8Rng};
use squishy_volumes_util::{Aabb, lambda, mu};

use crate::{PositionAndColliderBits, particle_parameters};

pub struct TestParticles {
    pub particle_masses: Vec<f32>,
    pub particle_initial_volumes: Vec<f32>,
    pub particle_parameters: Vec<particle_parameters::Device>,
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

pub fn test_lame_parameters() -> impl Iterator<Item = particle_parameters::Host> + Clone {
    use particle_parameters::{Host, Solid};

    [[10000., 0.3], [1000000., 0.3], [10000., 0.], [0., 0.4]]
        .into_iter()
        .map(|[youngs_modulus, poissons_ratio]| {
            let mu = mu(youngs_modulus, poissons_ratio);
            let lambda = lambda(youngs_modulus, poissons_ratio);
            Host::Solid(Solid {
                mu,
                lambda,
                viscosity: None,
                sand_alpha: None,
            })
        })
}

pub fn test_inviscid_parameters() -> impl Iterator<Item = particle_parameters::Host> + Clone {
    use particle_parameters::{Fluid, Host};

    [(100., 2), (1000., 2), (100., 7), (1000., 7)]
        .into_iter()
        .map(|(bulk_modulus, exponent)| {
            Host::Fluid(Fluid {
                exponent,
                bulk_modulus,
                viscosity: None,
            })
        })
}

impl TestParticles {
    pub fn new(num_particles: usize, aabb: Aabb<Vector3<f32>>, sampling: ParticleSampling) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(33);

        let particle_masses = (0..num_particles)
            .map(|_| rng.random_range(0.1..1.0))
            .collect();
        let particle_initial_volumes = (0..num_particles)
            .map(|_| rng.random_range(0.1..1.0))
            .collect();
        let particle_parameters = test_lame_parameters()
            .chain(test_inviscid_parameters())
            .cycle()
            .take(num_particles)
            .map(Into::into)
            .collect::<Vec<_>>();
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
            particle_masses,
            particle_initial_volumes,
            particle_parameters,
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        }
    }
}
