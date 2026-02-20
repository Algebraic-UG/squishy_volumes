// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::ops::{Deref, DerefMut};

use nalgebra::Vector3;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ColliderInfo {
    pub distance: T,
    pub normal: Vector3<T>,
    pub velocity: Vector3<T>,
    pub friction: T,
    pub stickyness: T,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct GridNodeCollider {
    pub infos: FxHashMap<usize, ColliderInfo>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct GridCollider(pub FxHashMap<Vector3<i32>, GridNodeCollider>);

impl Deref for GridCollider {
    type Target = FxHashMap<Vector3<i32>, GridNodeCollider>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GridCollider {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
