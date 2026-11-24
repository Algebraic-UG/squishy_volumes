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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    Continue,
    FrameTimeReached,
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ShaderError {
    #[error("exceeded table tries")]
    TableTriesExceeded,
    #[error("failed to find entry")]
    TableEntryMissing,
    #[error("exceeded indirect limit")]
    IndirectLimitExceeded,
    #[error("found a particle too close to inversion")]
    ParticleCloseToInverted,
    #[error("ended up with a zero time step")]
    TimeStepZero,
    #[error("found a degenerate (zero area) collider triangle")]
    DegenerateTriangle,
    #[error("reported unknown status bits {bits}")]
    Unknown { bits: u32 },
}

const TABLE_TRIES_EXCEEDED: u32 = 1;
const TABLE_ENTRY_MISSING: u32 = 2;
const INDIRECT_LIMIT_EXCEEDED: u32 = 4;
const PARTICLE_CLOSE_TO_INVERTED: u32 = 8;
const TIME_STEP_ZERO: u32 = 16;
const REACHED_FRAME_TIME: u32 = 32;
const DEGENERATE_TRIANGLE: u32 = 64;

impl GpuStatus {
    /// Any status bit besides the frame time is a failure, even if the frame time
    /// was reached as well.
    pub fn to_result(&self, context: &GpuContext) -> Result<StepOutcome, Error> {
        let bits = self.0 & 0xFFFF;
        match bits {
            0 => return Ok(StepOutcome::Continue),
            REACHED_FRAME_TIME => return Ok(StepOutcome::FrameTimeReached),
            _ => {}
        }

        let shader_id = self.0 >> 16;
        let reporting_shader = context
            .get_shader_label(shader_id)
            .ok_or(Error::ShaderIdMissing(shader_id))?;

        Err(Error::Shader {
            reporting_shader,
            source: bits.into(),
        })
    }
}

impl From<u32> for ShaderError {
    fn from(bits: u32) -> Self {
        let is_set = |flag| bits & flag != 0;
        if is_set(TABLE_ENTRY_MISSING) {
            Self::TableEntryMissing
        } else if is_set(TABLE_TRIES_EXCEEDED) {
            Self::TableTriesExceeded
        } else if is_set(INDIRECT_LIMIT_EXCEEDED) {
            Self::IndirectLimitExceeded
        } else if is_set(PARTICLE_CLOSE_TO_INVERTED) {
            Self::ParticleCloseToInverted
        } else if is_set(TIME_STEP_ZERO) {
            Self::TimeStepZero
        } else if is_set(DEGENERATE_TRIANGLE) {
            Self::DegenerateTriangle
        } else {
            Self::Unknown { bits }
        }
    }
}

impl AllowedInBinding for GpuStatus {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::get_shared_context;

    fn status_from(context: &mut GpuContext, label: &'static str, bits: u32) -> GpuStatus {
        GpuStatus(context.get_shader_id(label) << 16 | bits)
    }

    #[test]
    fn no_bits_continue() {
        let mut context = get_shared_context();
        let status = status_from(&mut context, "status_test", 0);
        assert!(matches!(
            status.to_result(&context),
            Ok(StepOutcome::Continue)
        ));
    }

    #[test]
    fn frame_time_bit_is_an_outcome() {
        let mut context = get_shared_context();
        let status = status_from(&mut context, "status_test", REACHED_FRAME_TIME);
        assert!(matches!(
            status.to_result(&context),
            Ok(StepOutcome::FrameTimeReached)
        ));
    }

    #[test]
    fn failure_bit_names_the_shader() {
        let mut context = get_shared_context();
        let status = status_from(
            &mut context,
            "status_test",
            TIME_STEP_ZERO | REACHED_FRAME_TIME,
        );
        assert!(matches!(
            status.to_result(&context),
            Err(Error::Shader {
                reporting_shader: "status_test",
                source: ShaderError::TimeStepZero,
            })
        ));
    }

    #[test]
    fn unknown_bit_is_a_failure() {
        let mut context = get_shared_context();
        let status = status_from(&mut context, "status_test", 128 | REACHED_FRAME_TIME);
        assert!(matches!(
            status.to_result(&context),
            Err(Error::Shader {
                reporting_shader: "status_test",
                source: ShaderError::Unknown { bits: 160 },
            })
        ));
    }
}
