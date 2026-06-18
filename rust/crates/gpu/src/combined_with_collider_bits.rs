// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;

use crate::AllowedInBinding;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PositionAndColliderBits {
    pub position: Vector3<f32>,
    pub collider_bits: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq, Eq, Hash, Debug)]
pub struct NodeIdAndColliderBits {
    pub node_id: Vector3<i32>,
    pub collider_bits: u32,
}

impl AllowedInBinding for PositionAndColliderBits {}
impl AllowedInBinding for NodeIdAndColliderBits {}
