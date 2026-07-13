// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::sync::Mutex;

use nalgebra::Vector3;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct GridKey {
    pub node_id: Vector3<i32>,
    pub collider_bits: u32,
}

#[derive(Default)]
pub struct GridNodes {
    pub map: FxHashMap<GridKey, u32>,

    pub multi_map: FxHashMap<Vector3<i32>, SmallVec<[u32; 3]>>,

    pub keys: Vec<GridKey>,

    pub contributors: Vec<Mutex<SmallVec<[u32; 16]>>>,

    pub masses: Vec<f32>,
    pub velocities: Vec<Vector3<f32>>,
}
