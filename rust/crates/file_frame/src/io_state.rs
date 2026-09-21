// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use squishy_volumes_util::AnimatedGlobals;

use super::*;

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct IoState {
    pub time: f64,

    pub animated_globals: AnimatedGlobals,

    pub particles: Particles,

    pub collider: Collider,

    // these can be computed from scratch, so they are optional
    pub bvh: Option<squishy_volumes_mesh_util::BoundingVolumeHierarchy>,
    pub grid_nodes: Option<GridNodes>,
}
