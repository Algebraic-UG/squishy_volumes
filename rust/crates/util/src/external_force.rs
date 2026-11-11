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

#[cfg(test)]
mod test;

use nalgebra::Vector3;

use crate::{AnimatedGlobals, NORMALIZATION_EPS, T};

/// The velocity after gravity, global damping and the goal spring acted for `time_step`.
///
/// `to_goal` is the offset to the particle's goal, if it has one. Damping and the goal
/// spring are integrated implicitly (backward Euler, with the spring evaluated where the
/// particle will be after the step), so neither can make the velocity grow or overshoot,
/// however stiff or damped they are. Goal damping only acts towards the goal.
/// Negative stiffness or damping is treated as zero.
pub fn apply_external_force(
    velocity: &Vector3<T>,
    to_goal: Option<&Vector3<T>>,
    globals: &AnimatedGlobals,
    time_step: T,
) -> Vector3<T> {
    let gravity = Vector3::new(globals.gravity_x, globals.gravity_y, globals.gravity_z).cast::<T>();
    let pushed = velocity + gravity * time_step;
    let damping_factor = 1. + time_step * (globals.damping as T).max(0.);

    let Some(to_goal) = to_goal else {
        return pushed / damping_factor;
    };

    let goal_stiffness = (globals.goal_stiffness as T).max(0.);
    let pulled = pushed + to_goal * (time_step * goal_stiffness);
    let spring_factor = damping_factor + time_step * time_step * goal_stiffness;

    let distance = to_goal.norm();
    if distance <= NORMALIZATION_EPS {
        return pulled / spring_factor;
    }
    let to_goal_direction = to_goal / distance;
    let towards_goal = to_goal_direction.dot(&pulled);
    let across = pulled - to_goal_direction * towards_goal;
    let goal_damping_factor = spring_factor + time_step * (globals.goal_damping as T).max(0.);

    across / spring_factor + to_goal_direction * (towards_goal / goal_damping_factor)
}
