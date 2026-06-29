// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix4x3, Vector4};

use crate::AllowedInBinding;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq, Default)]
pub struct Svd {
    pub u: Matrix4x3<f32>,
    pub s: Vector4<f32>,
    pub v: Matrix4x3<f32>,
}

impl AllowedInBinding for Svd {
    const ALIGNMENT: std::num::NonZeroU64 = Vector4::<f32>::ALIGNMENT;
}
