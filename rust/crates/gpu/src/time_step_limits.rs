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
