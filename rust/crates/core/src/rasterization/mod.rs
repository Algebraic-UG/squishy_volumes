// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use iter_enumeration::{IntoIterEnum2, IntoIterEnum3 as _};
use nalgebra::{Unit, Vector2, Vector3};
use squishy_volumes_api::T;
use std::{iter::empty, mem::swap};

use crate::{
    math::{Aabb, NORMALIZATION_EPS},
    state::grids::ColliderInfo,
};

pub enum Rasterized {
    Invalid(T),
    Valid(ColliderInfo),
}

impl Rasterized {
    pub fn distance_abs(&self) -> T {
        match self {
            Rasterized::Invalid(distance) => *distance,
            Rasterized::Valid(info) => info.distance.abs(),
        }
    }
}

pub struct RasterizationVertex<'a> {
    pub position: &'a Vector3<T>,
    pub velocity: &'a Vector3<T>,
    pub normal: &'a Option<Unit<Vector3<T>>>,
}

pub fn rasterize<'a>(
    spacing: T,
    layers: usize,
    [a, b, c]: [RasterizationVertex<'a>; 3],
    [d, e, f]: [Option<&Vector3<T>>; 3],
    friction: T,
    stickyness: T,
) -> impl Iterator<Item = (Vector3<i32>, Rasterized)> {
    let ab = a.position - b.position;
    let bc = b.position - c.position;
    let ca = c.position - a.position;

    let Some(normal) = (-ab).cross(&ca).try_normalize(NORMALIZATION_EPS) else {
        return empty().iter_enum_2a();
    };

    let mix_normal = |other: Vector3<T>| {
        other
            .try_normalize(NORMALIZATION_EPS)
            .and_then(|other| Unit::try_new((other + normal).scale(0.5), NORMALIZATION_EPS))
            .unwrap_or(Unit::new_unchecked(normal))
    };
    let normal_ab = d.map(|d| mix_normal(ab.cross(&(d - b.position))));
    let normal_bc = e.map(|e| mix_normal(bc.cross(&(e - c.position))));
    let normal_ca = f.map(|f| mix_normal(ca.cross(&(f - a.position))));

    candidates(a.position, b.position, c.position, &normal, spacing, layers)
        .filter_map(
            move |grid_node: Vector3<i32>| -> Option<(Vector3<i32>, Rasterized)> {
                let p = grid_node.map(|c| c as T * spacing);
                let sa = (p - b.position).dot(&normal.cross(&ab)) < 0.;
                let sb = (p - c.position).dot(&normal.cross(&bc)) < 0.;
                let sc = (p - a.position).dot(&normal.cross(&ca)) < 0.;

                if (sa && sb && sc) || (!sa && !sb && !sc) {
                    let distance = (p - a.position).dot(&normal);
                    (distance.abs() <= spacing * layers as T).then_some(Rasterized::Valid(
                        ColliderInfo {
                            distance,
                            normal,

                            //TODO
                            velocity: Vector3::zeros(),
                            friction,
                            stickyness,
                        },
                    ))
                } else {
                    let mut result: Option<Rasterized> = None;
                    let mut edge_contribution =
                        |start: &RasterizationVertex,
                         end: &RasterizationVertex,
                         edge_normal: &Option<Unit<Vector3<T>>>| {
                            let segment = end.position - start.position;
                            let along_segment =
                                (p - start.position).dot(&segment) / segment.norm_squared();

                            let element_normal;
                            let to_p;
                            if along_segment < 0. {
                                element_normal = start.normal;
                                to_p = p - start.position;
                            } else if along_segment > 1. {
                                element_normal = end.normal;
                                to_p = p - start.position - segment;
                            } else {
                                element_normal = edge_normal;
                                to_p = p - start.position - segment * along_segment;
                            }

                            let distance = to_p.norm();
                            if distance > spacing * layers as T {
                                return;
                            }

                            if result
                                .as_ref()
                                .is_some_and(|result| result.distance_abs() < distance)
                            {
                                return;
                            }

                            let Some(element_normal) = element_normal else {
                                result = Some(Rasterized::Invalid(distance));
                                return;
                            };

                            let sign = to_p.dot(element_normal).signum();
                            result = Some(Rasterized::Valid(ColliderInfo {
                                distance: distance * sign,
                                normal: to_p
                                    .try_normalize(NORMALIZATION_EPS)
                                    .map(|n| n * sign)
                                    .unwrap_or(**element_normal),

                                //TODO
                                velocity: Vector3::zeros(),
                                friction,
                                stickyness,
                            }));
                        };

                    edge_contribution(&b, &a, &normal_ab);
                    edge_contribution(&c, &b, &normal_bc);
                    edge_contribution(&a, &c, &normal_ca);

                    result
                }
                .map(|result| (grid_node, result))
            },
        )
        .iter_enum_2b()
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
