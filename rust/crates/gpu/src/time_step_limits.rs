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
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq, Default)]
pub struct TimeStepLimits {
    pub time_step_by_velocity: f32,
    pub time_step_by_deformation: f32,
    pub time_step_by_isolated: f32,
    pub time_step_by_sound: f32,
}

impl AllowedInBinding for TimeStepLimits {
    const ALIGNMENT: NonZeroU64 = f32::ALIGNMENT;
}
