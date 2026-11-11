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

use crate::{NORMALIZATION_EPS, T};

/// A particle that ended up on the wrong side of a collider surface.
pub struct Contact {
    /// Offset from the closest point on the surface to the particle.
    pub to_p: Vector3<T>,
    /// Velocity of the surface at the closest point.
    pub collider_velocity: Vector3<T>,
    pub friction: T,
    pub damping: T,
}

/// The parts of `relative_velocity` along the unit `contact_normal` and tangential to it.
pub fn split_normal_tangential(
    relative_velocity: &Vector3<T>,
    contact_normal: &Vector3<T>,
) -> (Vector3<T>, Vector3<T>) {
    let normal_velocity = contact_normal * relative_velocity.dot(contact_normal);
    (normal_velocity, relative_velocity - normal_velocity)
}

/// Impulse that slows sliding by `friction · correction_speed`, but never reverses it.
pub fn friction_impulse(
    tangential_velocity: &Vector3<T>,
    friction: T,
    correction_speed: T,
) -> Vector3<T> {
    let tangential_speed = tangential_velocity.norm();
    if tangential_speed <= NORMALIZATION_EPS {
        return Vector3::zeros();
    }
    tangential_velocity / tangential_speed * (friction * correction_speed).min(tangential_speed)
}

/// Impulse that removes the fraction `damping` of the normal velocity, at most all of it.
pub fn damping_impulse(normal_velocity: &Vector3<T>, damping: T) -> Vector3<T> {
    normal_velocity * damping.min(1.)
}

/// The velocity that moves the particle back onto the surface within one time step,
/// with friction and damping applied to its velocity relative to the surface.
/// A particle closer to the surface than [`NORMALIZATION_EPS`] has no contact normal
/// and keeps its velocity.
pub fn collision_response(velocity: &Vector3<T>, contact: &Contact, time_step: T) -> Vector3<T> {
    let distance = contact.to_p.norm();
    if distance < NORMALIZATION_EPS {
        return *velocity;
    }
    let contact_normal = contact.to_p / distance;
    let correction_speed = distance / time_step;
    let (normal_velocity, tangential_velocity) =
        split_normal_tangential(&(velocity - contact.collider_velocity), &contact_normal);
    velocity
        - friction_impulse(&tangential_velocity, contact.friction, correction_speed)
        - damping_impulse(&normal_velocity, contact.damping)
        - contact.to_p / time_step
}
