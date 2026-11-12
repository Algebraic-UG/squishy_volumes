// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::BTreeMap;

use nalgebra::{Matrix3, Vector3};
use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};
use squishy_volumes_util::{
    ParticleFlags, ParticleParameters, elastic_energy_inviscid, elastic_energy_neo_hookean,
};

use super::*;

const GRID_NODE_SIZES: [f32; 2] = [0.1, 1.];
const TIME_STEP: f32 = 1e-3;

fn test_positions(grid_node_size: f32) -> Vec<Vector3<f32>> {
    const ON_NODE_BETWEEN_AND_AT_STENCIL_SWITCH: [f32; 4] = [0., 0.25, 0.5, 0.75];
    let offsets_within_a_cell = ON_NODE_BETWEEN_AND_AT_STENCIL_SWITCH
        .into_iter()
        .flat_map(|x| {
            ON_NODE_BETWEEN_AND_AT_STENCIL_SWITCH
                .into_iter()
                .flat_map(move |y| {
                    ON_NODE_BETWEEN_AND_AT_STENCIL_SWITCH
                        .into_iter()
                        .map(move |z| Vector3::new(x, y, z))
                })
        });
    let cells = [Vector3::new(-3., -1., -2.), Vector3::new(2., 0., 5.)];
    cells
        .into_iter()
        .flat_map(|cell| {
            offsets_within_a_cell
                .clone()
                .map(move |offset| cell + offset)
        })
        .map(|normalized| normalized * grid_node_size)
        .collect()
}

fn stencil(position: &Vector3<f32>, grid_node_size: f32) -> Vec<(f32, Vector3<f32>)> {
    quadratic_stencil(position, grid_node_size)
        .map(|node_id| node_weight(&node_id, position, grid_node_size))
        .collect()
}

fn assert_vector_close(expected: &Vector3<f32>, actual: &Vector3<f32>, tolerance: f32) {
    assert!(
        (expected - actual).norm() <= tolerance * (1. + expected.norm()),
        "expected {expected} but got {actual}"
    );
}

#[test]
fn weights_are_a_partition_of_unity() {
    for grid_node_size in GRID_NODE_SIZES {
        for position in test_positions(grid_node_size) {
            let weights = stencil(&position, grid_node_size);
            assert!(weights.iter().all(|(weight, _)| *weight >= 0.));
            let sum: f32 = weights.iter().map(|(weight, _)| weight).sum();
            assert!((sum - 1.).abs() < 1e-5, "{position}: {sum}");
        }
    }
}

#[test]
fn weights_reproduce_linear_functions() {
    for grid_node_size in GRID_NODE_SIZES {
        for position in test_positions(grid_node_size) {
            let first_moment: Vector3<f32> = stencil(&position, grid_node_size)
                .iter()
                .map(|(weight, to_grid_node)| to_grid_node * *weight)
                .sum();
            assert!(first_moment.norm() < 1e-5 * grid_node_size, "{position}");
        }
    }
}

#[test]
fn second_moment_is_the_apic_inertia() {
    for grid_node_size in GRID_NODE_SIZES {
        for position in test_positions(grid_node_size) {
            let second_moment: Matrix3<f32> = stencil(&position, grid_node_size)
                .iter()
                .map(|(weight, to_grid_node)| to_grid_node * to_grid_node.transpose() * *weight)
                .sum();
            let inertia = Matrix3::identity() / quadratic_inverse_inertia(grid_node_size);
            assert!(
                (second_moment - inertia).norm() < 1e-5 * grid_node_size.powi(2),
                "{position}: {second_moment}"
            );
        }
    }
}

#[test]
fn stencil_covers_the_whole_support() {
    for grid_node_size in GRID_NODE_SIZES {
        for position in test_positions(grid_node_size) {
            let inside: Vec<_> = quadratic_stencil(&position, grid_node_size).collect();
            let center = (position / grid_node_size).map(|x| x.round() as i32);
            let neighbourhood = (-3..=3).flat_map(|i| {
                (-3..=3).flat_map(move |j| (-3..=3).map(move |k| Vector3::new(i, j, k)))
            });
            for offset in neighbourhood {
                let node_id = center + offset;
                if inside.contains(&node_id) {
                    continue;
                }
                let (weight, _) = node_weight(&node_id, &position, grid_node_size);
                assert_eq!(weight, 0., "{position}, {node_id}");
            }
        }
    }
}

struct Particle {
    position: Vector3<f32>,
    mass: f32,
    velocity: Vector3<f32>,
    velocity_gradient: Matrix3<f32>,
}

fn test_particles(grid_node_size: f32) -> Vec<Particle> {
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    (0..20)
        .map(|i| Particle {
            position: Vector3::from_fn(|_, _| rng.random_range(-0.5..0.5) * 4. * grid_node_size),
            mass: 0.5 + 0.1 * i as f32,
            velocity: Vector3::from_fn(|_, _| rng.random_range(-0.5..0.5)),
            velocity_gradient: Matrix3::from_fn(|_, _| rng.random_range(-0.5..0.5)),
        })
        .collect()
}

struct GridNode {
    position: Vector3<f32>,
    mass: f32,
    momentum: Vector3<f32>,
}

fn particles_to_grid(particles: &[Particle], grid_node_size: f32) -> Vec<GridNode> {
    let mut grid: BTreeMap<[i32; 3], (f32, Vector3<f32>)> = Default::default();
    for particle in particles {
        for node_id in quadratic_stencil(&particle.position, grid_node_size) {
            let (weight, to_grid_node) = node_weight(&node_id, &particle.position, grid_node_size);
            let momentum = affine_momentum(
                &to_grid_node,
                particle.mass,
                &particle.velocity,
                &particle.velocity_gradient,
            );
            let (mass, total) = grid.entry(node_id.into()).or_insert((0., Vector3::zeros()));
            *mass += weight * particle.mass;
            *total += momentum * weight;
        }
    }
    grid.into_iter()
        .map(|(node_id, (mass, momentum))| GridNode {
            position: Vector3::from(node_id).map(|x| x as f32) * grid_node_size,
            mass,
            momentum,
        })
        .collect()
}

fn angular_momentum_of_particles(particles: &[Particle], grid_node_size: f32) -> Vector3<f32> {
    let inertia = 1. / quadratic_inverse_inertia(grid_node_size);
    particles
        .iter()
        .map(|particle| {
            let c = &particle.velocity_gradient;
            let spin = Vector3::new(c.m32 - c.m23, c.m13 - c.m31, c.m21 - c.m12);
            particle
                .position
                .cross(&(particle.velocity * particle.mass))
                + spin * (particle.mass * inertia)
        })
        .sum()
}

fn grid_to_particle(
    grid: &[GridNode],
    position: &Vector3<f32>,
    grid_node_size: f32,
) -> (Vector3<f32>, Matrix3<f32>) {
    let grid_velocity_at = |node_position: &Vector3<f32>| {
        let node = grid
            .iter()
            .find(|node| (node.position - node_position).norm() < 1e-3 * grid_node_size)
            .unwrap();
        node.momentum / node.mass
    };
    let nodes = quadratic_stencil(position, grid_node_size).map(|node_id| {
        let (weight, to_grid_node) = node_weight(&node_id, position, grid_node_size);
        (
            weight,
            to_grid_node,
            grid_velocity_at(&(position + to_grid_node)),
        )
    });
    gather_from_nodes(nodes, grid_node_size)
}

#[test]
fn particles_to_grid_conserves_mass_and_momentum() {
    for grid_node_size in GRID_NODE_SIZES {
        let particles = test_particles(grid_node_size);
        let grid = particles_to_grid(&particles, grid_node_size);

        let grid_mass: f32 = grid.iter().map(|node| node.mass).sum();
        let particle_mass: f32 = particles.iter().map(|particle| particle.mass).sum();
        assert!((grid_mass - particle_mass).abs() < 1e-5 * particle_mass);

        let grid_momentum: Vector3<f32> = grid.iter().map(|node| node.momentum).sum();
        let particle_momentum: Vector3<f32> = particles
            .iter()
            .map(|particle| particle.velocity * particle.mass)
            .sum();
        assert_vector_close(&particle_momentum, &grid_momentum, 1e-5);
    }
}

#[test]
fn particles_to_grid_conserves_angular_momentum() {
    for grid_node_size in GRID_NODE_SIZES {
        let particles = test_particles(grid_node_size);
        let grid = particles_to_grid(&particles, grid_node_size);
        let grid_angular_momentum: Vector3<f32> = grid
            .iter()
            .map(|node| node.position.cross(&node.momentum))
            .sum();
        assert_vector_close(
            &angular_momentum_of_particles(&particles, grid_node_size),
            &grid_angular_momentum,
            1e-4,
        );
    }
}

#[test]
fn grid_to_particles_conserves_momentum_and_angular_momentum() {
    for grid_node_size in GRID_NODE_SIZES {
        let particles = test_particles(grid_node_size);
        let grid = particles_to_grid(&particles, grid_node_size);
        let gathered: Vec<Particle> = particles
            .iter()
            .map(|particle| {
                let (velocity, velocity_gradient) =
                    grid_to_particle(&grid, &particle.position, grid_node_size);
                Particle {
                    position: particle.position,
                    mass: particle.mass,
                    velocity,
                    velocity_gradient,
                }
            })
            .collect();

        let grid_momentum: Vector3<f32> = grid.iter().map(|node| node.momentum).sum();
        let gathered_momentum: Vector3<f32> = gathered
            .iter()
            .map(|particle| particle.velocity * particle.mass)
            .sum();
        assert_vector_close(&grid_momentum, &gathered_momentum, 1e-4);

        let grid_angular_momentum: Vector3<f32> = grid
            .iter()
            .map(|node| node.position.cross(&node.momentum))
            .sum();
        assert_vector_close(
            &grid_angular_momentum,
            &angular_momentum_of_particles(&gathered, grid_node_size),
            1e-4,
        );
    }
}

#[test]
fn affine_velocity_field_survives_a_round_trip() {
    let base_velocity = Vector3::new(0.3, -1.2, 0.5);
    let gradient = Matrix3::new(0.2, -0.7, 0.1, 0.7, -0.3, 0.4, 0.05, -0.4, 0.1);
    for grid_node_size in GRID_NODE_SIZES {
        let mut particles = test_particles(grid_node_size);
        for particle in &mut particles {
            particle.velocity = base_velocity + gradient * particle.position;
            particle.velocity_gradient = gradient;
        }
        let grid = particles_to_grid(&particles, grid_node_size);
        for particle in &particles {
            let (velocity, velocity_gradient) =
                grid_to_particle(&grid, &particle.position, grid_node_size);
            assert_vector_close(&particle.velocity, &velocity, 1e-4);
            assert!((velocity_gradient - gradient).norm() < 1e-4 * gradient.norm());
        }
    }
}

fn test_deformation() -> (Matrix3<f32>, Matrix3<f32>) {
    let position_gradient = Matrix3::new(1.1, 0.2, -0.1, 0.05, 0.9, 0.3, -0.2, 0.1, 1.05);
    let velocity_gradient = Matrix3::new(0.2, -0.7, 0.1, 0.3, -0.3, 0.4, 0.05, -0.4, 0.3);
    (position_gradient, velocity_gradient)
}

#[test]
fn internal_forces_neither_move_nor_spin_the_body() {
    let (position_gradient, velocity_gradient) = test_deformation();
    let parameters = ParticleParameters::default();
    let viscous = ParticleFlags::USE_VISCOSITY;
    for flags in [
        ParticleFlags::IS_SOLID,
        ParticleFlags::IS_FLUID,
        ParticleFlags::IS_SOLID | viscous,
        ParticleFlags::IS_FLUID | viscous,
    ] {
        let stress = kirchhoff_stress(flags, &parameters, &position_gradient, &velocity_gradient);
        assert!((stress - stress.transpose()).norm() < 1e-5 * stress.norm());

        for grid_node_size in GRID_NODE_SIZES {
            let stress_impulse = StressImpulse {
                kirchhoff_stress: &stress,
                initial_volume: parameters.initial_volume(),
                time_step: TIME_STEP,
                grid_node_size,
            };
            let impulse_scale =
                stress.norm() * TIME_STEP * parameters.initial_volume() / grid_node_size;
            for position in test_positions(grid_node_size) {
                let mut force = Vector3::zeros();
                let mut torque = Vector3::zeros();
                for (weight, to_grid_node) in stencil(&position, grid_node_size) {
                    let impulse = stress_impulse.towards(&to_grid_node) * weight;
                    force += impulse;
                    torque += (position + to_grid_node).cross(&impulse);
                }
                assert!(force.norm() < 1e-5 * impulse_scale, "{flags:?} {position}");
                let lever = position.norm() + grid_node_size;
                assert!(
                    torque.norm() < 1e-4 * lever * impulse_scale,
                    "{flags:?} {position}: {torque}"
                );
            }
        }
    }
}

fn stress_power_on_an_affine_grid_motion(
    stress_impulse: &StressImpulse,
    velocity_gradient: &Matrix3<f32>,
    position: &Vector3<f32>,
) -> f32 {
    stencil(position, stress_impulse.grid_node_size)
        .into_iter()
        .map(|(weight, to_grid_node)| {
            let force = -stress_impulse.towards(&to_grid_node) * (weight / TIME_STEP);
            let node_velocity = velocity_gradient * (position + to_grid_node);
            force.dot(&node_velocity)
        })
        .sum()
}

#[test]
fn stress_power_matches_the_elastic_energy_rate() {
    let (position_gradient, velocity_gradient) = test_deformation();
    let parameters = ParticleParameters::default();
    for flags in [ParticleFlags::IS_SOLID, ParticleFlags::IS_FLUID] {
        let energy = |position_gradient: &Matrix3<f32>| {
            if flags.contains(ParticleFlags::IS_SOLID) {
                elastic_energy_neo_hookean(parameters.mu(), parameters.lambda(), position_gradient)
            } else {
                elastic_energy_inviscid(
                    parameters.bulk_modulus,
                    parameters.exponent,
                    position_gradient,
                )
            }
        };
        let step = 1e-2;
        let advanced =
            |time: f32| (Matrix3::identity() + velocity_gradient * time) * position_gradient;
        let energy_rate = (energy(&advanced(step)) - energy(&advanced(-step))) / (2. * step);
        let expected_power = -parameters.initial_volume() * energy_rate;

        let stress = kirchhoff_stress(flags, &parameters, &position_gradient, &velocity_gradient);
        for grid_node_size in GRID_NODE_SIZES {
            let stress_impulse = StressImpulse {
                kirchhoff_stress: &stress,
                initial_volume: parameters.initial_volume(),
                time_step: TIME_STEP,
                grid_node_size,
            };
            for position in test_positions(grid_node_size) {
                let power = stress_power_on_an_affine_grid_motion(
                    &stress_impulse,
                    &velocity_gradient,
                    &position,
                );
                assert!(
                    (power - expected_power).abs() < 1e-2 * expected_power.abs(),
                    "{flags:?} {position}: {power} instead of {expected_power}"
                );
            }
        }
    }
}

#[test]
fn viscous_power_is_dissipation_in_the_current_volume() {
    let (position_gradient, velocity_gradient) = test_deformation();
    let expanded = position_gradient * 1.3;
    let current_volume_ratio = expanded.determinant();
    assert!((current_volume_ratio - 1.).abs() > 1.);

    let purely_viscous = ParticleParameters {
        youngs_modulus: 0.,
        ..Default::default()
    };
    let rate_of_strain = (velocity_gradient + velocity_gradient.transpose()) / 2.;
    let divergence = velocity_gradient.trace();
    let deviatoric_rate_of_strain =
        rate_of_strain - Matrix3::from_diagonal_element(divergence / 3.);
    let dissipation_density =
        2. * purely_viscous.viscosity_dynamic * deviatoric_rate_of_strain.norm_squared()
            + purely_viscous.viscosity_bulk * divergence.powi(2);
    let current_volume = current_volume_ratio * purely_viscous.initial_volume();
    let expected_power = -current_volume * dissipation_density;

    let stress = kirchhoff_stress(
        ParticleFlags::IS_SOLID | ParticleFlags::USE_VISCOSITY,
        &purely_viscous,
        &expanded,
        &velocity_gradient,
    );
    for grid_node_size in GRID_NODE_SIZES {
        let stress_impulse = StressImpulse {
            kirchhoff_stress: &stress,
            initial_volume: purely_viscous.initial_volume(),
            time_step: TIME_STEP,
            grid_node_size,
        };
        for position in test_positions(grid_node_size) {
            let power = stress_power_on_an_affine_grid_motion(
                &stress_impulse,
                &velocity_gradient,
                &position,
            );
            assert!(power <= 0., "{position}: {power}");
            assert!(
                (power - expected_power).abs() < 1e-4 * expected_power.abs(),
                "{position}: {power} instead of {expected_power}"
            );
        }
    }
}
