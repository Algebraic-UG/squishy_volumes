// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use iter_enumeration::IntoIterEnum2;
use nalgebra::{Vector2, Vector3};
use squishy_volumes_api::T;
use std::iter::empty;
use tracing::info;

use crate::math::Aabb;

pub fn rasterize(
    corner_a: &Vector3<T>,
    corner_b: &Vector3<T>,
    corner_c: &Vector3<T>,
    normal: &Vector3<T>,
    spacing: T,
    layers: usize,
) -> impl Iterator<Item = Vector3<T>> {
    let offset = normal.dot(corner_a);
    let margin = spacing * layers as T;
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
    let to_world = move |point: &Vector2<T>, dist: T| -> Vector3<T> {
        match facing_axis {
            0 => Vector3::new(dist, point.x, point.y),
            1 => Vector3::new(point.x, dist, point.y),
            2 => Vector3::new(point.x, point.y, dist),
            _ => unreachable!(),
        }
    };

    let a = to_plane(corner_a);
    let b = to_plane(corner_b);
    let c = to_plane(corner_c);
    let n = to_plane(normal);

    let ab = a - b;
    let bc = b - c;
    let ca = c - a;

    let ab_ns = ab.norm_squared();
    let bc_ns = bc.norm_squared();
    let ca_ns = ca.norm_squared();

    if ab_ns == 0. || bc_ns == 0. || ca_ns == 0. {
        empty::<Vector3<T>>().iter_enum_2a()
    } else {
        let mut projected_aabb = Aabb::new([a, b, c].into_iter());
        projected_aabb.min -= Vector2::repeat(margin);
        projected_aabb.max += Vector2::repeat(margin);
        let (num, lattice) = projected_aabb.lattice(spacing);
        info!(num);

        lattice
            .filter(move |ray_start| {
                let inside_triangle = |p: &Vector2<T>| -> bool {
                    let sa = (p - b).perp(&ab).is_sign_negative();
                    let sb = (p - c).perp(&bc).is_sign_negative();
                    let sc = (p - a).perp(&ca).is_sign_negative();
                    (sa && sb && sc) || (!sa && !sb && !sc)
                };

                let distance_to_segment = |start: &Vector2<T>,
                                           segment: &Vector2<T>,
                                           squared_norm: T,
                                           point: &Vector2<T>|
                 -> T {
                    let factor = ((point - start).dot(segment) / squared_norm).clamp(0., 1.);
                    let projected = start + factor * segment;
                    (projected - point).norm()
                };

                inside_triangle(ray_start)
                    || distance_to_segment(&b, &ab, ab_ns, ray_start) < margin
                    || distance_to_segment(&c, &bc, bc_ns, ray_start) < margin
                    || distance_to_segment(&a, &ca, ca_ns, ray_start) < margin
            })
            .flat_map(move |ray_start| {
                let hit_0 = (offset - margin - n.dot(&ray_start)) / normal_facing_coord;
                let hit_1 = (offset + margin - n.dot(&ray_start)) / normal_facing_coord;

                let n = (((hit_0 - hit_1) / spacing).abs().floor() as usize).max(1);
                (0..n + 1)
                    .map(move |i| {
                        let factor = i as T / n as T;
                        hit_0 * factor + hit_1 * (1. - factor)
                    })
                    .map(move |coord| to_world(&ray_start, coord))
            })
            .iter_enum_2b()
    }
}
