// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use iter_enumeration::IntoIterEnum2;
use nalgebra::{Unit, Vector3};
use squishy_volumes_api::T;
use std::iter::empty;

use squishy_volumes_util::{NORMALIZATION_EPS, rasterization::candidates};

use crate::state::grids::{ColliderInfo, Rasterized};

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
) -> impl Iterator<Item = (Vector3<i32>, Rasterized)> {
    let ab = a.position - b.position;
    let bc = b.position - c.position;
    let ca = c.position - a.position;

    let normal_area_2 = (-ab).cross(&ca);
    let area_2 = normal_area_2.norm();
    if area_2 < NORMALIZATION_EPS {
        return empty().iter_enum_2a();
    }
    let normal = normal_area_2 / area_2;

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

                let bary_c = (p - b.position).dot(&normal.cross(&ab)) / area_2;
                let bary_a = (p - c.position).dot(&normal.cross(&bc)) / area_2;
                let bary_b = (p - a.position).dot(&normal.cross(&ca)) / area_2;

                let velocity = -*a.velocity * bary_a - b.velocity * bary_b - c.velocity * bary_c;

                let sa = bary_a < 0.;
                let sb = bary_b < 0.;
                let sc = bary_c < 0.;

                if (sa && sb && sc) || (!sa && !sb && !sc) {
                    let distance = (p - a.position).dot(&normal);
                    (distance.abs() <= spacing * layers as T).then_some(Rasterized::Valid(
                        ColliderInfo {
                            distance,
                            normal,

                            velocity,
                            friction,
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

                                velocity,
                                friction,
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
