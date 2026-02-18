// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use iter_enumeration::IntoIterEnum3 as _;
use nalgebra::{Vector2, Vector3};
use squishy_volumes_api::T;
use std::{iter::empty, mem::swap};

use crate::math::{Aabb, NORMALIZATION_EPS};

pub fn rasterize(
    corner_a: &Vector3<T>,
    corner_b: &Vector3<T>,
    corner_c: &Vector3<T>,
    spacing: T,
    layers: usize,
) -> impl Iterator<Item = Vector3<T>> {
    let Some(normal) = (corner_b - corner_a)
        .cross(&(corner_c - corner_a))
        .try_normalize(NORMALIZATION_EPS)
    else {
        return empty::<Vector3<T>>().iter_enum_3a();
    };

    let facing_axis = normal
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .unwrap()
        .0;
    let normal_facing_coord = normal[facing_axis];

    let to_plane = |corner: &Vector3<T>| -> Vector2<T> {
        match facing_axis {
            0 => Vector2::new(corner.y, corner.z),
            1 => Vector2::new(corner.x, corner.z),
            2 => Vector2::new(corner.x, corner.y),
            _ => unreachable!(),
        }
    };
    let to_world = move |point: &Vector2<i32>, final_coord: i32| -> Vector3<i32> {
        match facing_axis {
            0 => Vector3::new(final_coord, point.x, point.y),
            1 => Vector3::new(point.x, final_coord, point.y),
            2 => Vector3::new(point.x, point.y, final_coord),
            _ => unreachable!(),
        }
    };

    let candidates = {
        let a = to_plane(corner_a);
        let b = to_plane(corner_b);
        let c = to_plane(corner_c);
        let n = to_plane(&normal);

        let ab = a - b;
        let bc = b - c;
        let ca = c - a;

        let ab_ns = ab.norm_squared();
        let bc_ns = bc.norm_squared();
        let ca_ns = ca.norm_squared();

        if ab_ns == 0. || bc_ns == 0. || ca_ns == 0. {
            return empty::<Vector3<T>>().iter_enum_3b();
        }

        let aabb = Aabb::new([a, b, c].into_iter());
        let min = aabb
            .min
            .map(|c| (c / spacing).floor() as i32 - layers as i32);
        let max = aabb
            .max
            .map(|c| (c / spacing).ceil() as i32 + layers as i32);
        let offset = normal.dot(corner_a);
        let height_margin = layers as T * spacing;
        let side_margin_squared = ((layers + 1) as T * spacing).powi(2);

        (min.x..=max.x)
            .flat_map(move |i| (min.y..=max.y).map(move |j| Vector2::new(i, j)))
            .filter(move |projected_grid_node| {
                let p = projected_grid_node.map(|c| c as T * spacing);

                let sa = (p - b).perp(&ab) < 0.;
                let sb = (p - c).perp(&bc) < 0.;
                let sc = (p - a).perp(&ca) < 0.;
                if (sa && sb && sc) || (!sa && !sb && !sc) {
                    return true;
                }

                let distance_to_segment =
                    |start: &Vector2<T>, segment: &Vector2<T>, squared_norm: T| -> T {
                        let factor = ((p - start).dot(segment) / squared_norm).clamp(0., 1.);
                        let projected = start + factor * segment;
                        (projected - p).norm_squared()
                    };

                distance_to_segment(&b, &ab, ab_ns) < side_margin_squared
                    || distance_to_segment(&c, &bc, bc_ns) < side_margin_squared
                    || distance_to_segment(&a, &ca, ca_ns) < side_margin_squared
            })
            .flat_map(move |projected_grid_node| {
                let p = projected_grid_node.map(|c| c as T * spacing);
                let mut hit_0 = (offset - height_margin - n.dot(&p)) / normal_facing_coord;
                let mut hit_1 = (offset + height_margin - n.dot(&p)) / normal_facing_coord;

                if hit_1 < hit_0 {
                    swap(&mut hit_0, &mut hit_1);
                }

                let min = (hit_0 / spacing).floor() as i32;
                let max = (hit_1 / spacing).ceil() as i32;

                (min..=max).map(move |coord| to_world(&projected_grid_node, coord))
            })
            .map(move |v| v.map(move |c| spacing * c as T))
    };

    let ab = corner_a - corner_b;
    let bc = corner_b - corner_c;
    let ca = corner_c - corner_a;

    let ab_ns = ab.norm_squared();
    let bc_ns = bc.norm_squared();
    let ca_ns = ca.norm_squared();

    // Can't be zero here
    if ab_ns == 0. || bc_ns == 0. || ca_ns == 0. {
        panic!();
    }

    candidates
        .into_iter()
        .filter(move |candidate| {
            let sa = (candidate - corner_b).dot(&normal.cross(&ab)) < 0.;
            let sb = (candidate - corner_c).dot(&normal.cross(&bc)) < 0.;
            let sc = (candidate - corner_a).dot(&normal.cross(&ca)) < 0.;

            let distance = if (sa && sb && sc) || (!sa && !sb && !sc) {
                (candidate - corner_a).dot(&normal)
            } else {
                let distance_to_segment =
                    |start: &Vector3<T>, segment: &Vector3<T>, squared_norm: T| -> T {
                        let factor =
                            ((candidate - start).dot(segment) / squared_norm).clamp(0., 1.);
                        let projected = start + factor * segment;
                        (projected - candidate).norm_squared()
                    };

                [
                    distance_to_segment(corner_b, &ab, ab_ns),
                    distance_to_segment(corner_c, &bc, bc_ns),
                    distance_to_segment(corner_a, &ca, ca_ns),
                ]
                .iter()
                .min_by(|a, b| a.total_cmp(b))
                .map(|v| v.sqrt())
                .unwrap()
            };

            distance.abs() < spacing * layers as T
        })
        .iter_enum_3c()
}
