// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use iter_enumeration::{IntoIterEnum2 as _, IntoIterEnum3 as _};
use nalgebra::{Vector2, Vector3};
use squishy_volumes_api::T;
use std::{iter::empty, mem::swap};
use tracing::info;

use crate::{
    math::{Aabb, NORMALIZATION_EPS},
    state::grids::WeightedDistance,
};

pub fn rasterize(
    a: &Vector3<T>,
    b: &Vector3<T>,
    c: &Vector3<T>,
    spacing: T,
    layers: usize,
) -> impl Iterator<Item = (Vector3<i32>, WeightedDistance)> {
    let Some(normal) = (b - a).cross(&(c - a)).try_normalize(NORMALIZATION_EPS) else {
        return empty().iter_enum_3a();
    };

    let ab = a - b;
    let bc = b - c;
    let ca = c - a;

    let ab_ns = ab.norm_squared();
    let bc_ns = bc.norm_squared();
    let ca_ns = ca.norm_squared();

    if ab_ns == 0. || bc_ns == 0. || ca_ns == 0. {
        return empty().iter_enum_3b();
    }

    info!(candidates = candidates(a, b, c, &normal, spacing, layers).count());
    candidates(a, b, c, &normal, spacing, layers)
        .filter_map(
            move |grid_node: Vector3<i32>| -> Option<(Vector3<i32>, WeightedDistance)> {
                let p = grid_node.map(|c| c as T * spacing);
                let sa = (p - b).dot(&normal.cross(&ab)) < 0.;
                let sb = (p - c).dot(&normal.cross(&bc)) < 0.;
                let sc = (p - a).dot(&normal.cross(&ca)) < 0.;

                if (sa && sb && sc) || (!sa && !sb && !sc) {
                    let distance = (p - a).dot(&normal);
                    (distance.abs() <= spacing * layers as T).then_some(WeightedDistance {
                        distance,
                        sign_confidence: 1.,
                        normal,
                    })
                } else {
                    let to_ab = b + ((p - b).dot(&ab) / ab_ns).clamp(0., 1.) * ab - p;
                    let to_bc = c + ((p - c).dot(&bc) / bc_ns).clamp(0., 1.) * bc - p;
                    let to_ca = a + ((p - a).dot(&ca) / ca_ns).clamp(0., 1.) * ca - p;

                    let (distance_squared, direction) = [
                        (to_ab.norm_squared(), to_ab),
                        (to_bc.norm_squared(), to_bc),
                        (to_ca.norm_squared(), to_ca),
                    ]
                    .into_iter()
                    .min_by(|a, b| a.0.total_cmp(&b.0))
                    .unwrap();

                    (distance_squared <= (spacing * layers as T).powi(2)).then_some({
                        let distance = distance_squared.sqrt();
                        let dn = direction.dot(&normal);

                        if distance > 0. {
                            WeightedDistance {
                                distance: -(distance * dn.signum()),
                                sign_confidence: dn.abs() / distance,
                                normal: direction / distance * dn.signum(),
                            }
                        } else {
                            WeightedDistance {
                                distance,
                                sign_confidence: 0.,
                                normal,
                            }
                        }
                    })
                }
                .map(|weighted_distance| (grid_node, weighted_distance))
            },
        )
        .iter_enum_3c()
}

fn candidates(
    a: &Vector3<T>,
    b: &Vector3<T>,
    c: &Vector3<T>,
    n: &Vector3<T>,
    spacing: T,
    layers: usize,
) -> impl Iterator<Item = Vector3<i32>> + use<> {
    let offset = n.dot(a);
    let facing_axis = n
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .unwrap()
        .0;
    let normal_facing_coord = n[facing_axis];

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

    let a = to_plane(a);
    let b = to_plane(b);
    let c = to_plane(c);
    let n = to_plane(n);

    let ab = a - b;
    let bc = b - c;
    let ca = c - a;

    let ab_ns = ab.norm_squared();
    let bc_ns = bc.norm_squared();
    let ca_ns = ca.norm_squared();

    if ab_ns == 0. || bc_ns == 0. || ca_ns == 0. {
        return empty().iter_enum_2a();
    }

    let aabb = Aabb::new([a, b, c].into_iter());
    let min = aabb
        .min
        .map(|c| (c / spacing).floor() as i32 - layers as i32);
    let max = aabb
        .max
        .map(|c| (c / spacing).ceil() as i32 + layers as i32);
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
        .iter_enum_2b()
}
