// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix4x3, Vector4};
use squishy_volumes_util::triangle::{Opposites, Triangle};
use std::num::NonZeroU64;
use std::sync::atomic::AtomicU32;

use crate::{Block, Indirect};

pub trait AllowedInBinding: Sized {
    const MIN_BINDING_SIZE: NonZeroU64 = NonZeroU64::new(size_of::<Self>() as u64).unwrap();
    const ALIGNMENT: NonZeroU64 = NonZeroU64::new(size_of::<Self>() as u64).unwrap();
}

impl AllowedInBinding for u32 {}
impl AllowedInBinding for AtomicU32 {}
impl AllowedInBinding for f32 {}
impl AllowedInBinding for Vector4<f32> {}
impl AllowedInBinding for Vector4<i32> {}
impl AllowedInBinding for Vector4<u32> {}
impl AllowedInBinding for Matrix4x3<f32> {
    const ALIGNMENT: NonZeroU64 = Vector4::<f32>::ALIGNMENT;
}
impl AllowedInBinding for Block {
    const ALIGNMENT: NonZeroU64 = Vector4::<f32>::ALIGNMENT;
}
impl AllowedInBinding for Indirect {}

impl AllowedInBinding for Triangle {
    const ALIGNMENT: std::num::NonZeroU64 = u32::ALIGNMENT;
}

impl AllowedInBinding for Opposites {
    const ALIGNMENT: std::num::NonZeroU64 = u32::ALIGNMENT;
}
