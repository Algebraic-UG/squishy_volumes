// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use crate::AllowedInBinding;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq, Default)]
pub struct GpuStatus(u32);

const TABLE_TRIES_EXCEEDED: u32 = 1;
const TABLE_ENTRY_MISSING: u32 = 2;

impl GpuStatus {
    pub fn table_tries_exceeded(&self) -> bool {
        self.0 & TABLE_TRIES_EXCEEDED != 0
    }

    pub fn table_entry_missing(&self) -> bool {
        self.0 & TABLE_ENTRY_MISSING != 0
    }

    pub fn shader_id(&self) -> u32 {
        self.0 >> 16
    }
}

impl AllowedInBinding for GpuStatus {}
