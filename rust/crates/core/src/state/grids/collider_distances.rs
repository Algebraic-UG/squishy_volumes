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
use small_map::FxSmallMap;
use squishy_volumes_api::T;

use crate::state::grids::Mutex;

#[derive(Clone, Serialize, Deserialize)]
pub enum Rasterized {
    Invalid(T),
    Valid(ColliderInfo),
}

impl Rasterized {
    pub fn assume_valid(&self) -> &ColliderInfo {
        let Self::Valid(info) = self else {
            panic!("Invalid collider info");
        };
        info
    }
}

impl Rasterized {
    pub fn distance_abs(&self) -> T {
        match self {
            Rasterized::Invalid(distance) => *distance,
            Rasterized::Valid(info) => info.distance.abs(),
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ColliderInfo {
    pub distance: T,
    pub normal: Vector3<T>,
    pub velocity: Vector3<T>,
    pub friction: T,
}

pub type ColliderInfos = FxSmallMap<4, u8, Rasterized>;

#[derive(Default, Clone, Serialize, Deserialize)]
pub enum GridNodeCollider {
    #[default]
    Tmp,
    Mut(Mutex<ColliderInfos>),
    Ref(ColliderInfos),
}

impl GridNodeCollider {
    pub fn assume_ref(&self) -> &ColliderInfos {
        let Self::Ref(infos) = &self else {
            panic!("Collider node wasn't ref");
        };
        infos
    }

    pub fn assume_mut(&self) -> &Mutex<ColliderInfos> {
        let Self::Mut(infos) = &self else {
            panic!("Collider node wasn't mut");
        };
        infos
    }
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
