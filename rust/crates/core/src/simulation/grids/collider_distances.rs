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

use super::Mutex;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct WeightedDistance {
    pub distance: T,
    pub normal: Vector3<T>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct GridNodeColliderDistances {
    pub weighted_distances: FxHashMap<usize, WeightedDistance>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct GridColliderDistances(pub FxHashMap<Vector3<i32>, Mutex<GridNodeColliderDistances>>);

impl Deref for GridColliderDistances {
    type Target = FxHashMap<Vector3<i32>, Mutex<GridNodeColliderDistances>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GridColliderDistances {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
