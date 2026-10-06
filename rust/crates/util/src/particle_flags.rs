// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[repr(C)]
#[derive(
    Clone,
    Copy,
    bytemuck::Zeroable,
    bytemuck::Pod,
    Debug,
    PartialEq,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ParticleFlags(u32);

bitflags::bitflags! {
    impl ParticleFlags: u32{
        const IS_SOLID = 1 << 0;
        const IS_FLUID = 1 << 1;
        const USE_VISCOSITY = 1 << 2;
        const USE_SAND_ALPHA = 1 << 3;
        const HAS_GOAL = 1 << 4;
        const TOMBSTONED = 1 << 5;
        const FAILED = 1 << 6;
    }
}
