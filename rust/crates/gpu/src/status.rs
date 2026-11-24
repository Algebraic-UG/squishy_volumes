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

use crate::{AllowedInBinding, Error, GpuContext};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq, Default)]
pub struct GpuStatus(u32);

#[cfg(test)]
impl GpuStatus {
    pub fn reached_frame_time() -> Self {
        Self(REACHED_FRAME_TIME)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ShaderError {
    #[error("{reporting_shader} exceeded table tries")]
    TableTriesExceeded { reporting_shader: &'static str },
    #[error("{reporting_shader} failed to find entry")]
    TableEntryMissing { reporting_shader: &'static str },
    #[error("{reporting_shader} exceeded indirect limit")]
    IndirectLimitExceeded { reporting_shader: &'static str },
    #[error("{reporting_shader} found a particle too close to inversion")]
    ParticleCloseToInverted { reporting_shader: &'static str },
    #[error("{reporting_shader} ended up with a zero time step")]
    TimeStepZero { reporting_shader: &'static str },
    #[error("next frame is ready")]
    FrameTimeReached,
    #[error("{reporting_shader} found a degenerate (zero area) collider triangle")]
    DegenerateTriangle { reporting_shader: &'static str },
    #[error("{reporting_shader} reported unknown status bits {error}")]
    Unknown {
        reporting_shader: &'static str,
        error: u32,
    },
}

const TABLE_TRIES_EXCEEDED: u32 = 1;
const TABLE_ENTRY_MISSING: u32 = 2;
const INDIRECT_LIMIT_EXCEEDED: u32 = 4;
const PARTICLE_CLOSE_TO_INVERTED: u32 = 8;
const TIME_STEP_ZERO: u32 = 16;
const REACHED_FRAME_TIME: u32 = 32;
const DEGENERATE_TRIANGLE: u32 = 64;

impl GpuStatus {
    pub fn to_result(&self, context: &GpuContext) -> Result<(), Error> {
        let error = self.0 & 0xFFFF;
        if error == 0 {
            return Ok(());
        }

        let shader_id = self.0 >> 16;

        let Some(reporting_shader) = context.get_shader_label(shader_id) else {
            return Err(Error::ShaderIdMissing(shader_id));
        };

        if self.0 & TABLE_ENTRY_MISSING != 0 {
            Err(ShaderError::TableEntryMissing { reporting_shader })?;
        }

        if self.0 & TABLE_TRIES_EXCEEDED != 0 {
            Err(ShaderError::TableTriesExceeded { reporting_shader })?;
        }

        if self.0 & INDIRECT_LIMIT_EXCEEDED != 0 {
            Err(ShaderError::IndirectLimitExceeded { reporting_shader })?;
        }

        if self.0 & PARTICLE_CLOSE_TO_INVERTED != 0 {
            Err(ShaderError::ParticleCloseToInverted { reporting_shader })?;
        }

        if self.0 & TIME_STEP_ZERO != 0 {
            Err(ShaderError::TimeStepZero { reporting_shader })?;
        }

        if self.0 & DEGENERATE_TRIANGLE != 0 {
            Err(ShaderError::DegenerateTriangle { reporting_shader })?;
        }

        if self.0 & REACHED_FRAME_TIME != 0 {
            Err(ShaderError::FrameTimeReached)?;
        }

        Err(ShaderError::Unknown {
            reporting_shader,
            error,
        })?
    }
}

impl AllowedInBinding for GpuStatus {}
