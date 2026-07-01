// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use thiserror::Error;

use crate::{AllowedInBinding, GpuContext, GpuError};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq, Default)]
pub struct GpuStatus(u32);

#[derive(Error, Debug)]
pub enum GpuShaderError {
    #[error("{reporting_shader} exceeded table tries")]
    TableTriesExceeded { reporting_shader: &'static str },
    #[error("{reporting_shader} failed to find entry")]
    TableEntryMissing { reporting_shader: &'static str },
    #[error("{reporting_shader} exceeded indirect limit")]
    IndirectLimitExceeded { reporting_shader: &'static str },
}

const TABLE_TRIES_EXCEEDED: u32 = 1;
const TABLE_ENTRY_MISSING: u32 = 2;
const INDIRECT_LIMIT_EXCEEDED: u32 = 4;

impl GpuStatus {
    pub fn to_result(&self, context: &GpuContext) -> Result<(), GpuError> {
        let shader_id = self.0 >> 16;

        let Some(reporting_shader) = context.get_shader_label(shader_id) else {
            return Err(GpuError::ShaderIdMissing(shader_id));
        };

        if self.0 & TABLE_ENTRY_MISSING != 0 {
            Err(GpuShaderError::TableEntryMissing { reporting_shader })?;
        }

        if self.0 & TABLE_TRIES_EXCEEDED != 0 {
            Err(GpuShaderError::TableTriesExceeded { reporting_shader })?;
        }

        if self.0 & INDIRECT_LIMIT_EXCEEDED != 0 {
            Err(GpuShaderError::IndirectLimitExceeded { reporting_shader })?;
        }

        Ok(())
    }
}

impl AllowedInBinding for GpuStatus {}
