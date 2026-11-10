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
    ParticleFlags, ParticleParameters, fluid_plasticity, limit_time_step_by_isolated_particles,
    principal_stretches,
};

use super::*;

struct Particle {
    flags: ParticleFlags,
    parameters: ParticleParameters,
    position: Vector3<f32>,
    velocity: Vector3<f32>,
    velocity_gradient: Matrix3<f32>,
    position_gradient: Matrix3<f32>,
}

fn step(particles: &mut [Particle], time_step: f32, grid_node_size: f32) {
    let mut grid: BTreeMap<[i32; 3], (f32, Vector3<f32>)> = BTreeMap::new();
    for particle in particles.iter() {
        let stress = kirchhoff_stress(
            particle.flags,
            &particle.parameters,
            &particle.position_gradient,
            &particle.velocity_gradient,
        );
        let stress_impulse = StressImpulse {
            kirchhoff_stress: &stress,
            initial_volume: particle.parameters.initial_volume(),
            time_step,
            grid_node_size,
        };
        for node_id in quadratic_stencil(&particle.position, grid_node_size) {
            let (weight, to_grid_node) = node_weight(&node_id, &particle.position, grid_node_size);
            let momentum = affine_momentum(
                &to_grid_node,
                particle.parameters.mass(),
                &particle.velocity,
                &particle.velocity_gradient,
            ) - stress_impulse.towards(&to_grid_node);
            let (mass, total) = grid.entry(node_id.into()).or_insert((0., Vector3::zeros()));
            *mass += weight * particle.parameters.mass();
            *total += momentum * weight;
        }
    }
    for particle in particles.iter_mut() {
        let nodes = quadratic_stencil(&particle.position, grid_node_size).map(|node_id| {
            let (weight, to_grid_node) = node_weight(&node_id, &particle.position, grid_node_size);
            let (mass, momentum) = grid[&<[i32; 3]>::from(node_id)];
            (weight, to_grid_node, momentum / mass)
        });
        (particle.velocity, particle.velocity_gradient) = gather_from_nodes(nodes, grid_node_size);
        particle.position += particle.velocity * time_step;
        particle.position_gradient +=
            particle.velocity_gradient * particle.position_gradient * time_step;
        if particle.flags.contains(ParticleFlags::IS_FLUID) {
            particle.position_gradient = fluid_plasticity(&particle.position_gradient);
        }
    }
}

fn deviation(particles: &[Particle], time_step: f32) -> f32 {
    particles
        .iter()
        .map(|particle| {
            let has_stiffness = particle.flags.contains(ParticleFlags::IS_FLUID)
                || particle.parameters.youngs_modulus > 0.;
            let strain = if has_stiffness {
                (particle.position_gradient - Matrix3::identity()).norm()
            } else {
                0.
            };
            let rate = particle.velocity_gradient.norm() * time_step;
            if strain.is_finite() && rate.is_finite() {
                strain + rate
            } else {
                f32::INFINITY
            }
        })
        .fold(0., f32::max)
}

fn adaptive_time_step(particles: &[Particle], factor: f32, grid_node_size: f32) -> f32 {
    factor
        * particles
            .iter()
            .map(|particle| {
                limit_time_step_by_isolated_particles(
                    &particle.flags,
                    &particle.parameters,
                    &principal_stretches(&particle.position_gradient),
                    grid_node_size,
                )
            })
            .fold(f32::MAX, f32::min)
}

fn growth(mut particles: Vec<Particle>, factor: f32, grid_node_size: f32) -> f32 {
    let initial_time_step = adaptive_time_step(&particles, factor, grid_node_size);
    let initial = deviation(&particles, initial_time_step);
    let mut largest: f32 = 0.;
    for _ in 0..STEPS {
        let time_step = adaptive_time_step(&particles, factor, grid_node_size);
        step(&mut particles, time_step, grid_node_size);
        largest = largest.max(deviation(&particles, time_step));
        if !largest.is_finite() {
            break;
        }
    }
    largest / initial
}

const STEPS: usize = 200;
const PERTURBATION: f32 = 1e-4;

fn shear_mode() -> Matrix3<f32> {
    Matrix3::new(0., 1., 0., 1., 0., 0., 0., 0., 0.) * PERTURBATION
}

fn volume_mode() -> Matrix3<f32> {
    Matrix3::identity() * PERTURBATION
}

fn isolated(
    flags: ParticleFlags,
    parameters: ParticleParameters,
    strain: Matrix3<f32>,
    rate: Matrix3<f32>,
    grid_node_size: f32,
) -> Vec<Particle> {
    vec![Particle {
        flags,
        parameters,
        position: Vector3::new(0.31, 0.47, 0.12) * grid_node_size,
        velocity: Vector3::zeros(),
        velocity_gradient: rate,
        position_gradient: Matrix3::identity() + strain,
    }]
}

fn pseudo_random_symmetric(seed: usize) -> Matrix3<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed as u64);
    let matrix = Matrix3::from_fn(|_, _| rng.random_range(-0.5..0.5));
    (matrix + matrix.transpose()) * PERTURBATION
}

fn block(
    flags: ParticleFlags,
    parameters: ParticleParameters,
    perturb_strain: bool,
    grid_node_size: f32,
) -> Vec<Particle> {
    let spacing = grid_node_size / 2.;
    let per_side = 4;
    (0..per_side * per_side * per_side)
        .map(|index| {
            let cell = Vector3::new(
                index % per_side,
                index / per_side % per_side,
                index / per_side / per_side,
            );
            let perturbation = pseudo_random_symmetric(index);
            let (strain, rate) = if perturb_strain {
                (perturbation, Matrix3::zeros())
            } else {
                (Matrix3::zeros(), perturbation)
            };
            Particle {
                flags,
                parameters,
                position: cell.map(|x| x as f32 * spacing + 0.13 * spacing),
                velocity: Vector3::zeros(),
                velocity_gradient: rate,
                position_gradient: Matrix3::identity() + strain,
            }
        })
        .collect()
}

const GRID_NODE_SIZE: f32 = 0.1;
const STABLE_FACTOR: f32 = 0.95;
const UNSTABLE_FACTOR: f32 = 1.1;
const GROWTH_LIMIT: f32 = 100.;

struct Material {
    flags: ParticleFlags,
    parameters: ParticleParameters,
    has_stiffness: bool,
}

fn material(youngs_modulus: f32, viscosity_dynamic: f32, viscosity_bulk: f32) -> Material {
    let use_viscosity = viscosity_dynamic > 0. || viscosity_bulk > 0.;
    Material {
        flags: if use_viscosity {
            ParticleFlags::IS_SOLID | ParticleFlags::USE_VISCOSITY
        } else {
            ParticleFlags::IS_SOLID
        },
        parameters: ParticleParameters {
            initial_size: GRID_NODE_SIZE / 2.,
            youngs_modulus,
            poissons_ratio: 0.3,
            viscosity_dynamic,
            viscosity_bulk,
            ..Default::default()
        },
        has_stiffness: youngs_modulus > 0.,
    }
}

fn solids() -> [Material; 1] {
    [material(1e5, 0., 0.)]
}

fn isolated_in_each_mode(material: &Material) -> impl Iterator<Item = Vec<Particle>> + '_ {
    [shear_mode(), volume_mode()].into_iter().map(|mode| {
        let (strain, rate) = if material.has_stiffness {
            (mode, Matrix3::zeros())
        } else {
            (Matrix3::zeros(), mode)
        };
        isolated(
            material.flags,
            material.parameters,
            strain,
            rate,
            GRID_NODE_SIZE,
        )
    })
}

#[test]
fn isolated_particles_are_stable_below_the_limit() {
    for material in solids() {
        for particles in isolated_in_each_mode(&material) {
            let growth = growth(particles, STABLE_FACTOR, GRID_NODE_SIZE);
            assert!(growth < GROWTH_LIMIT, "{:?}: {growth}", material.parameters);
        }
    }
}

#[test]
fn isolated_particles_are_unstable_above_the_limit() {
    for material in solids() {
        let largest_growth = isolated_in_each_mode(&material)
            .map(|particles| growth(particles, UNSTABLE_FACTOR, GRID_NODE_SIZE))
            .fold(0., f32::max);
        assert!(
            largest_growth >= GROWTH_LIMIT,
            "{:?}: {largest_growth}",
            material.parameters
        );
    }
}

#[test]
fn isolated_fluids_are_stable_below_the_limit() {
    let parameters = ParticleParameters {
        initial_size: GRID_NODE_SIZE / 2.,
        ..Default::default()
    };
    let particles = isolated(
        ParticleFlags::IS_FLUID,
        parameters,
        volume_mode(),
        Matrix3::zeros(),
        GRID_NODE_SIZE,
    );
    assert!(growth(particles, STABLE_FACTOR, GRID_NODE_SIZE) < GROWTH_LIMIT);
}

#[test]
fn blocks_of_particles_are_stable_below_the_isolated_limit() {
    for material in solids() {
        let particles = block(
            material.flags,
            material.parameters,
            material.has_stiffness,
            GRID_NODE_SIZE,
        );
        let growth = growth(particles, STABLE_FACTOR, GRID_NODE_SIZE);
        assert!(growth < GROWTH_LIMIT, "{:?}: {growth}", material.parameters);
    }
}
