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

use super::*;

const TIME_STEPS: [T; 4] = [1e-4, 1e-3, 1e-2, 1e-1];

fn globals(
    gravity_z: f32,
    goal_stiffness: f32,
    goal_damping: f32,
    damping: f32,
) -> AnimatedGlobals {
    AnimatedGlobals {
        gravity_x: 0.,
        gravity_y: 0.,
        gravity_z,
        goal_stiffness,
        goal_damping,
        damping,
    }
}

struct Particle {
    position: Vector3<T>,
    velocity: Vector3<T>,
}

impl Particle {
    fn step(&mut self, goal: &Vector3<T>, globals: &AnimatedGlobals, time_step: T) {
        let to_goal = goal - self.position;
        self.velocity = apply_external_force(&self.velocity, Some(&to_goal), globals, time_step);
        self.position += self.velocity * time_step;
    }

    fn energy(&self, goal: &Vector3<T>, goal_stiffness: T) -> T {
        0.5 * self.velocity.norm_squared()
            + 0.5 * goal_stiffness * (goal - self.position).norm_squared()
    }
}

#[test]
fn gravity_alone_accelerates_uniformly() {
    let globals = globals(-9.8, 0., 0., 0.);
    let velocity = Vector3::new(1., -2., 0.5);
    for time_step in TIME_STEPS {
        let expected = velocity + Vector3::new(0., 0., -9.8) * time_step;
        let actual = apply_external_force(&velocity, None, &globals, time_step);
        assert!((expected - actual).norm() < 1e-5, "{expected} vs {actual}");
    }
}

#[test]
fn global_damping_never_reverses_the_velocity() {
    let velocity = Vector3::new(1., -2., 0.5);
    for damping in [0.1, 1., 10., 1e3, 1e6] {
        let globals = globals(0., 0., 0., damping);
        for time_step in TIME_STEPS {
            let damped = apply_external_force(&velocity, None, &globals, time_step);
            let ratio = damped.dot(&velocity) / velocity.norm_squared();
            assert!(
                (0. ..=1.).contains(&ratio),
                "{damping} {time_step}: {ratio}"
            );
            assert!((damped - velocity * ratio).norm() < 1e-5);
        }
    }
}

#[test]
fn global_damping_decays_exponentially() {
    let damping: f32 = 3.;
    let globals = globals(0., 0., 0., damping);
    let duration: T = 1.;
    let time_step = 1e-4;
    let mut velocity = Vector3::new(1., -2., 0.5);
    let initial = velocity;
    for _ in 0..(duration / time_step).round() as usize {
        velocity = apply_external_force(&velocity, None, &globals, time_step);
    }
    let expected = initial * (-T::from(damping) * duration).exp();
    assert!(
        (expected - velocity).norm() < 1e-3 * initial.norm(),
        "{expected} vs {velocity}"
    );
}

#[test]
fn goal_spring_follows_the_damped_oscillator() {
    let goal_stiffness: f32 = 1000.;
    let goal_damping: f32 = 2.;
    let globals = globals(0., goal_stiffness, goal_damping, 0.);
    let goal = Vector3::new(0.3, -0.1, 0.2);
    let initial_offset = 0.05;
    let direction = Vector3::new(1., 2., -2.) / 3.;
    let mut particle = Particle {
        position: goal + direction * initial_offset,
        velocity: Vector3::zeros(),
    };

    let time_step = 1e-5;
    let duration: T = 0.3;
    for _ in 0..(duration / time_step).round() as usize {
        particle.step(&goal, &globals, time_step);
    }

    let decay = T::from(goal_damping) / 2.;
    let frequency = (T::from(goal_stiffness) - decay * decay).sqrt();
    let expected_offset = initial_offset
        * (-decay * duration).exp()
        * ((frequency * duration).cos() + decay / frequency * (frequency * duration).sin());
    let expected = goal + direction * expected_offset;
    assert!(
        (expected - particle.position).norm() < 1e-2 * initial_offset,
        "{expected} vs {}",
        particle.position
    );
}

#[test]
fn stiff_goal_springs_never_gain_energy() {
    let goal = Vector3::new(0.3, -0.1, 0.2);
    for goal_stiffness in [1e3, 1e5, 1e7, 1e9] {
        for goal_damping in [0., 0.5, 1e3] {
            let globals = globals(0., goal_stiffness, goal_damping, 0.);
            for time_step in TIME_STEPS {
                let mut particle = Particle {
                    position: goal + Vector3::new(0.05, -0.02, 0.01),
                    velocity: Vector3::new(0.4, 0.1, -0.3),
                };
                let mut energy = particle.energy(&goal, T::from(goal_stiffness));
                for _ in 0..100 {
                    particle.step(&goal, &globals, time_step);
                    let next = particle.energy(&goal, T::from(goal_stiffness));
                    assert!(
                        next <= energy * (1. + 1e-4),
                        "k {goal_stiffness} d {goal_damping} dt {time_step}: {energy} -> {next}"
                    );
                    energy = next;
                }
            }
        }
    }
}

#[test]
fn goal_damping_leaves_motion_across_the_goal_direction_alone() {
    let globals = globals(0., 0., 1e4, 0.);
    let to_goal = Vector3::new(0., 0., 0.1);
    let velocity = Vector3::new(0.7, -0.4, 0.3);
    for time_step in TIME_STEPS {
        let updated = apply_external_force(&velocity, Some(&to_goal), &globals, time_step);
        assert!((updated.xy() - velocity.xy()).norm() < 1e-6);
        assert!(0. < updated.z && updated.z < velocity.z);
    }
}

#[test]
fn goal_spring_is_linear_in_the_offset_even_at_the_goal() {
    let goal_stiffness: f32 = 1e4;
    let damping: f32 = 1.;
    let time_step: T = 1e-2;
    let globals = globals(-9.8, goal_stiffness, 0., damping);
    let goal_stiffness = T::from(goal_stiffness);
    let damping = T::from(damping);
    let velocity = Vector3::new(0.7, -0.4, 0.3);
    let at_goal = apply_external_force(&velocity, Some(&Vector3::zeros()), &globals, time_step);
    let spring_factor = 1. + time_step * damping + time_step * time_step * goal_stiffness;
    for offset in [
        Vector3::new(1e-7, 0., 0.),
        Vector3::new(0., -2e-5, 1e-5),
        Vector3::new(0.1, 0.2, -0.3),
    ] {
        let expected = at_goal + offset * (time_step * goal_stiffness / spring_factor);
        let near_goal = apply_external_force(&velocity, Some(&offset), &globals, time_step);
        assert!((expected - near_goal).norm() < 1e-5, "{offset}");
    }
}

#[test]
fn negative_stiffness_and_damping_act_as_zero() {
    let velocity = Vector3::new(0.7, -0.4, 0.3);
    let to_goal = Vector3::new(0.1, 0.2, -0.3);
    let time_step = 1e-2;
    let without = apply_external_force(
        &velocity,
        Some(&to_goal),
        &globals(-9.8, 0., 0., 0.),
        time_step,
    );
    for (goal_stiffness, goal_damping, damping) in [
        (-1e4, 0., 0.),
        (0., -1e3, 0.),
        (0., 0., -100.),
        (-1e4, -1e3, -150.),
    ] {
        let negative = globals(-9.8, goal_stiffness, goal_damping, damping);
        let actual = apply_external_force(&velocity, Some(&to_goal), &negative, time_step);
        assert_eq!(without, actual);
    }
}
