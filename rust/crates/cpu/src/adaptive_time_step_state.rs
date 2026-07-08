// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(Default)]
pub struct AdaptiveTimeStepState {
    pub time_step_by_velocity: Option<f32>,
    pub time_step_by_deformation: Option<f32>,
    pub time_step_by_isolated: Option<f32>,
    pub time_step_by_sound: Option<f32>,
    pub time_step_prior: std::collections::VecDeque<f32>,
}

impl AdaptiveTimeStepState {
    pub fn allowed_time_step(&self, max_time_step: f32) -> f32 {
        [
            max_time_step,
            self.time_step_by_velocity.unwrap_or(f32::MAX),
            self.time_step_by_deformation.unwrap_or(f32::MAX),
            self.time_step_by_sound.unwrap_or(f32::MAX),
            self.time_step_by_isolated.unwrap_or(f32::MAX),
        ]
        .into_iter()
        .chain(self.time_step_prior.iter().cloned())
        .min_by(f32::total_cmp)
        .unwrap()
    }
}
