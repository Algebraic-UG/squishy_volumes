// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use rustc_hash::FxHashMap;
use squishy_volumes_api::T;

use crate::state::grids::GridNodeColliderDistances;

pub fn check_shifted_quadratic(shifted: Vector3<T>) -> bool {
    shifted.x >= 0.5
        && shifted.x <= 1.5
        && shifted.y >= 0.5
        && shifted.y <= 1.5
        && shifted.z >= 0.5
        && shifted.z <= 1.5
}

#[allow(unused)]
pub fn check_shifted_cubic(shifted: Vector3<T>) -> bool {
    shifted.x >= 1.
        && shifted.x <= 2.
        && shifted.y >= 1.
        && shifted.y <= 2.
        && shifted.z >= 1.
        && shifted.z <= 2.
}

pub fn find_worst_incompatibility(
    collider_insides: &FxHashMap<usize, bool>,
    grid_node: &GridNodeColliderDistances,
) -> Option<usize> {
    collider_insides
        .iter()
        .filter_map(|(collider_idx, inside)| {
            Some((
                *collider_idx,
                grid_node
                    .weighted_distances
                    .get(collider_idx)
                    .and_then(|weighted_distance| {
                        (inside ^ (weighted_distance.distance < 0.))
                            .then_some(weighted_distance.distance.abs())
                    })?,
            ))
        })
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(collider_idx, _)| collider_idx)
}
