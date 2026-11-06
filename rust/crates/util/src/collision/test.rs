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

use nalgebra::Vector3;
use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};

use super::*;

const TIME_STEP: T = 1e-2;
const TOLERANCE: T = 1e-5;

fn test_vectors() -> Vec<Vector3<T>> {
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    (0..50)
        .map(|_| Vector3::from_fn(|_, _| rng.random_range(-2. ..2.)))
        .collect()
}

struct Encounter {
    velocity: Vector3<T>,
    collider_velocity: Vector3<T>,
    to_p: Vector3<T>,
}

fn test_encounters() -> Vec<Encounter> {
    let vectors = test_vectors();
    (0..vectors.len())
        .map(|i| Encounter {
            velocity: vectors[i],
            collider_velocity: vectors[(i + 1) % vectors.len()],
            to_p: vectors[(i + 2) % vectors.len()] * 0.01,
        })
        .collect()
}

fn respond(encounter: &Encounter, friction: T, damping: T) -> Vector3<T> {
    collision_response(
        &encounter.velocity,
        &Contact {
            to_p: encounter.to_p,
            collider_velocity: encounter.collider_velocity,
            friction,
            damping,
        },
        TIME_STEP,
    )
}

#[test]
fn split_is_orthogonal_and_complete() {
    let vectors = test_vectors();
    for (velocity, normal) in vectors.iter().zip(vectors.iter().rev()) {
        let normal = normal.normalize();
        let (normal_part, tangential_part) = split_normal_tangential(velocity, &normal);
        assert!((normal_part + tangential_part - velocity).norm() < TOLERANCE);
        assert!(tangential_part.dot(&normal).abs() < TOLERANCE);
        assert!(normal_part.cross(&normal).norm() < TOLERANCE);
    }
}

#[test]
fn friction_slows_sliding_without_reversing_it() {
    for tangential_velocity in test_vectors() {
        for friction in [0., 0.3, 1., 5.] {
            for correction_speed in [0., 0.1, 1., 10.] {
                let after = tangential_velocity
                    - friction_impulse(&tangential_velocity, friction, correction_speed);
                let expected_speed =
                    (tangential_velocity.norm() - friction * correction_speed).max(0.);
                assert!((after.norm() - expected_speed).abs() < TOLERANCE);
                assert!(after.dot(&tangential_velocity) >= -TOLERANCE);
            }
        }
    }
}

#[test]
fn resting_particle_is_moved_onto_the_surface() {
    for to_p in test_vectors() {
        let resting = Encounter {
            velocity: Vector3::zeros(),
            collider_velocity: Vector3::zeros(),
            to_p,
        };
        let velocity = respond(&resting, 0.3, 0.1);
        assert!((velocity * TIME_STEP + to_p).norm() < TOLERANCE * (1. + to_p.norm()));
    }
}

#[test]
fn only_the_relative_velocity_matters() {
    let offset = Vector3::new(3., -1., 0.5);
    for encounter in test_encounters() {
        let moved = Encounter {
            velocity: encounter.velocity + offset,
            collider_velocity: encounter.collider_velocity + offset,
            to_p: encounter.to_p,
        };
        for (friction, damping) in [(0., 0.), (0.3, 0.1), (1., 1.)] {
            let reference = respond(&encounter, friction, damping);
            let response = respond(&moved, friction, damping);
            assert!((response - offset - reference).norm() < TOLERANCE * (1. + reference.norm()));
        }
    }
}

#[test]
fn full_damping_leaves_only_the_correction_in_normal_direction() {
    for encounter in test_encounters() {
        let normal = encounter.to_p.normalize();
        let after = respond(&encounter, 0., 1.);
        let relative_normal_speed = (after - encounter.collider_velocity).dot(&normal);
        let correction_speed = -encounter.to_p.norm() / TIME_STEP;
        assert!((relative_normal_speed - correction_speed).abs() < 100. * TOLERANCE);
    }
}

#[test]
fn particle_on_the_surface_keeps_its_velocity() {
    for encounter in test_encounters() {
        let on_surface = Encounter {
            to_p: encounter.to_p.normalize() * NORMALIZATION_EPS * 0.5,
            ..encounter
        };
        let response = respond(&on_surface, 0.3, 0.1);
        assert_eq!(response, on_surface.velocity);
        assert!(response.iter().all(|x| x.is_finite()));
    }
}
