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

use nalgebra::{Matrix4, Matrix4x3, Vector4};
use squishy_volumes_mesh_util::{Opposites, Triangle};
use squishy_volumes_util::{AnimatedGlobals, ParticleFlags, ParticleParameters};
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
impl AllowedInBinding for Matrix4<f32> {
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

impl AllowedInBinding for AnimatedGlobals {
    const ALIGNMENT: std::num::NonZeroU64 = f32::ALIGNMENT;
}
impl AllowedInBinding for ParticleFlags {}
impl AllowedInBinding for ParticleParameters {
    const ALIGNMENT: NonZeroU64 = u32::ALIGNMENT;
}
