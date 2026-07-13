// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::iter::once;

const TIME_STEP_HISTORY_LENGTH: usize = 10;

pub struct AdaptiveTimeStepState {
    pub max_time_step: f32,
    pub time_step_by_velocity: Option<f32>,
    pub time_step_by_deformation: Option<f32>,
    pub time_step_by_isolated: Option<f32>,
    pub time_step_by_sound: Option<f32>,
    pub time_step_prior: std::collections::VecDeque<f32>,
}

impl Default for AdaptiveTimeStepState {
    fn default() -> Self {
        Self {
            max_time_step: f32::MAX,
            time_step_by_velocity: Default::default(),
            time_step_by_deformation: Default::default(),
            time_step_by_isolated: Default::default(),
            time_step_by_sound: Default::default(),
            time_step_prior: Default::default(),
        }
    }
}

impl AdaptiveTimeStepState {
    fn allowed_time_step_without_prior(&self) -> f32 {
        [
            self.max_time_step,
            self.time_step_by_velocity.unwrap_or(f32::MAX),
            self.time_step_by_deformation.unwrap_or(f32::MAX),
            self.time_step_by_sound.unwrap_or(f32::MAX),
            self.time_step_by_isolated.unwrap_or(f32::MAX),
        ]
        .into_iter()
        .min_by(f32::total_cmp)
        .unwrap()
    }

    pub fn allowed_time_step(&self) -> f32 {
        once(self.allowed_time_step_without_prior())
            .chain(self.time_step_prior.iter().cloned())
            .min_by(f32::total_cmp)
            .unwrap()
    }

    pub fn push_current_limit(&mut self) {
        if self.time_step_prior.len() > TIME_STEP_HISTORY_LENGTH {
            self.time_step_prior.pop_front();
        }

        self.time_step_prior
            .push_back(self.allowed_time_step_without_prior());
    }
}
