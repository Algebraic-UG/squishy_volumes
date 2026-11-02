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

#[repr(C)]
#[derive(
    Clone,
    Copy,
    bytemuck::Zeroable,
    bytemuck::Pod,
    Debug,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct AnimatedGlobals {
    pub gravity_x: f32,
    pub gravity_y: f32,
    pub gravity_z: f32,
    pub goal_stiffness: f32,
    pub goal_damping: f32,
    pub damping: f32,
}

impl Default for AnimatedGlobals {
    fn default() -> Self {
        Self {
            gravity_x: 0.,
            gravity_y: 0.,
            gravity_z: -9.8,
            goal_stiffness: 1000.,
            goal_damping: 0.5,
            damping: 0.,
        }
    }
}

impl AnimatedGlobals {
    pub fn interpolate(&self, other: &Self, factor: f32) -> Self {
        Self {
            gravity_x: (1. - factor) * self.gravity_x + factor * other.gravity_x,
            gravity_y: (1. - factor) * self.gravity_y + factor * other.gravity_y,
            gravity_z: (1. - factor) * self.gravity_z + factor * other.gravity_z,
            goal_stiffness: (1. - factor) * self.goal_stiffness + factor * other.goal_stiffness,
            goal_damping: (1. - factor) * self.goal_damping + factor * other.goal_damping,
            damping: (1. - factor) * self.damping + factor * other.damping,
        }
    }
}
