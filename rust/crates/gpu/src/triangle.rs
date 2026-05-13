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
pub struct Triangle {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

impl AllowedInBinding for Triangle {
    const ALIGNMENT: std::num::NonZeroU64 = u32::ALIGNMENT;
}
