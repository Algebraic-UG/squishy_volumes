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
