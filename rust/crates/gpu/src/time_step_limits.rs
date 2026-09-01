// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU64;

use crate::AllowedInBinding;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq)]
pub struct TimeStepLimits {
    pub time_step_by_velocity: f32,
    pub time_step_by_deformation: f32,
    pub time_step_by_isolated: f32,
    pub time_step_by_sound: f32,
}

impl Default for TimeStepLimits {
    fn default() -> Self {
        Self {
            time_step_by_velocity: f32::MAX,
            time_step_by_deformation: f32::MAX,
            time_step_by_isolated: f32::MAX,
            time_step_by_sound: f32::MAX,
        }
    }
}

impl TimeStepLimits {
    pub fn min(&self, other: &Self) -> Self {
        Self {
            time_step_by_velocity: self.time_step_by_velocity.min(other.time_step_by_velocity),
            time_step_by_deformation: self
                .time_step_by_deformation
                .min(other.time_step_by_deformation),
            time_step_by_isolated: self.time_step_by_isolated.min(other.time_step_by_isolated),
            time_step_by_sound: self.time_step_by_sound.min(other.time_step_by_sound),
        }
    }

    pub fn time_step(&self) -> f32 {
        self.time_step_by_deformation
            .min(self.time_step_by_isolated)
            .min(self.time_step_by_sound)
            .min(self.time_step_by_velocity)
    }
}

impl AllowedInBinding for TimeStepLimits {
    const ALIGNMENT: NonZeroU64 = f32::ALIGNMENT;
}
