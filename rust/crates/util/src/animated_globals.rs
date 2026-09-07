// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[repr(C)]
#[derive(
    Default,
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
