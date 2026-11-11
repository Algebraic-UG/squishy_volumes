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

use nalgebra::Vector3;
use squishy_volumes_util::NORMALIZATION_EPS;

use crate::{DistanceResult, segment_distance_result};

/// Unit normal of the counter-clockwise triangle, zero if it is degenerate.
pub fn triangle_normal(a: &Vector3<f32>, b: &Vector3<f32>, c: &Vector3<f32>) -> Vector3<f32> {
    (b - a)
        .cross(&(c - a))
        .try_normalize(NORMALIZATION_EPS)
        .unwrap_or(Vector3::zeros())
}

/// Pseudo normal of a vertex: the normals of its incident triangles weighted by
/// their angle at the vertex, see "Signed Distance Computation Using the Angle
/// Weighted Pseudonormal" (Bærentzen, Aanæs). Each incident item holds the two
/// other corners and the normal of one triangle.
pub fn angle_weighted_normal(
    vertex: &Vector3<f32>,
    incident: impl Iterator<Item = (Vector3<f32>, Vector3<f32>, Vector3<f32>)>,
) -> Vector3<f32> {
    incident
        .map(|(a, b, normal)| (a - vertex).angle(&(b - vertex)) * normal)
        .sum::<Vector3<f32>>()
        .try_normalize(NORMALIZATION_EPS)
        .unwrap_or(Vector3::zeros())
}

/// Pseudo normal of an edge: the sum of the normals of the two triangles sharing it,
/// zero for a boundary edge without a neighbour.
pub fn edge_pseudo_normal(
    normal: &Vector3<f32>,
    neighbour_normal: Option<&Vector3<f32>>,
) -> Vector3<f32> {
    neighbour_normal.map_or(Vector3::zeros(), |neighbour_normal| {
        normal + neighbour_normal
    })
}

/// Barycentric coordinates of the projection of `p` onto the plane of the
/// triangle with unit normal `n`. They sum to one and are all positive inside.
pub fn barycentric_coordinates(
    p: &Vector3<f32>,
    [a, b, c]: [&Vector3<f32>; 3],
    n: &Vector3<f32>,
) -> Vector3<f32> {
    let ab = a - b;
    let bc = b - c;
    let ca = c - a;
    let area2_abc = n.dot(&ca.cross(&ab));
    Vector3::new(
        n.dot(&bc.cross(&(c - p))),
        n.dot(&ca.cross(&(a - p))),
        n.dot(&ab.cross(&(b - p))),
    ) / area2_abc
}

/// Pseudo normals of the corners `a`, `b`, `c` and the edges `ab`, `bc`, `ca`.
pub struct FeatureNormals {
    pub vertices: [Vector3<f32>; 3],
    pub edges: [Vector3<f32>; 3],
}

/// The closest point on the triangle to `p`, as the offset from it to `p`,
/// together with the pseudo normal of the face, edge or corner it lies on.
pub fn closest_feature(
    p: &Vector3<f32>,
    [a, b, c]: [&Vector3<f32>; 3],
    n: &Vector3<f32>,
    FeatureNormals {
        vertices: [a_n, b_n, c_n],
        edges: [ab_n, bc_n, ca_n],
    }: &FeatureNormals,
) -> DistanceResult {
    let barycentric = barycentric_coordinates(p, [a, b, c], n);
    if barycentric.iter().all(|x| *x > 0.) {
        return DistanceResult {
            distance: (p - a).dot(n).abs(),
            to_p: n * (p - a).dot(n),
            normal: *n,
        };
    }
    [
        segment_distance_result(p, a, b, a_n, ab_n, b_n),
        segment_distance_result(p, b, c, b_n, bc_n, c_n),
        segment_distance_result(p, c, a, c_n, ca_n, a_n),
    ]
    .into_iter()
    .min_by(|a, b| a.distance.total_cmp(&b.distance))
    .unwrap()
}
