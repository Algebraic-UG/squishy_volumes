// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use crate::triangle::{Opposites, Triangle};

pub fn compute_triangle_lists(
    num_vertices: usize,
    triangle_indices: &[Triangle],
) -> Vec<SmallVec<[u32; 8]>> {
    let mut vertex_to_triangles: Vec<SmallVec<[u32; 8]>> = vec![Default::default(); num_vertices];
    for (triangle_index, indices) in triangle_indices.iter().enumerate() {
        for vertex_index in indices.iter() {
            vertex_to_triangles[*vertex_index as usize].push(triangle_index as u32);
        }
    }
    vertex_to_triangles
        .iter_mut()
        .enumerate()
        .for_each(|(this_vertex, triangles)| {
            let mut neighbor_counts: FxHashMap<u32, u8> = Default::default();
            for triangle_index in triangles.iter() {
                for &vertex_index in triangle_indices[*triangle_index as usize].iter() {
                    if vertex_index != this_vertex as u32 {
                        *neighbor_counts.entry(vertex_index).or_default() += 1;
                    }
                }
            }
            assert!(neighbor_counts.values().all(|&count| count <= 2));
            if neighbor_counts.into_values().any(|count| count != 2) {
                triangles.clear();
            }
        });
    // this is so that range calculation via prefix sum can be used
    // TODO: maybe that should happen at the place of use, then
    vertex_to_triangles.push(Default::default());
    vertex_to_triangles
}

pub fn compute_triangle_opposites(triangle_indices: &[Triangle]) -> Vec<Opposites> {
    let order_edge = |[a, b]: [u32; 2]| if a < b { [a, b] } else { [b, a] };
    let mut edge_to_triangle: FxHashMap<[u32; 2], SmallVec<[u32; 2]>> = Default::default();
    for (index, indices) in triangle_indices.iter().enumerate() {
        for (a, b) in indices.into_iter().zip(indices.into_iter().cycle().skip(1)) {
            edge_to_triangle
                .entry(order_edge([a, b]))
                .or_default()
                .push(index as u32);
        }
    }
    assert!(edge_to_triangle.values().all(|indices| indices.len() <= 2));
    triangle_indices
        .iter()
        .enumerate()
        .map(|(index, indices)| -> Opposites {
            indices
                .into_iter()
                .zip(indices.into_iter().cycle().skip(1))
                .map(|(a, b)| -> u32 {
                    edge_to_triangle
                        .get(&order_edge([a, b]))
                        .unwrap()
                        .iter()
                        .cloned()
                        .find(|&other| other != index as u32)
                        .unwrap_or(u32::MAX)
                })
                .into()
        })
        .collect()
}

pub struct DistanceResult {
    pub distance: f32,
    pub to_p: Vector3<f32>,
    pub normal: Vector3<f32>,
}

pub fn segment_distance_result(
    p: &Vector3<f32>,
    start: &Vector3<f32>,
    end: &Vector3<f32>,
    start_normal: &Vector3<f32>,
    segment_normal: &Vector3<f32>,
    end_normal: &Vector3<f32>,
) -> DistanceResult {
    let segment = end - start;
    let along_segment = (p - start).dot(&segment) / segment.dot(&segment);
    if along_segment < 0. {
        DistanceResult {
            distance: (p - start).norm(),
            to_p: p - start,
            normal: *start_normal,
        }
    } else if along_segment < 1. {
        DistanceResult {
            distance: (p - start - segment * along_segment).norm(),
            to_p: p - start - segment * along_segment,
            normal: *segment_normal,
        }
    } else {
        DistanceResult {
            distance: (p - end).norm(),
            to_p: p - end,
            normal: *end_normal,
        }
    }
}

fn distance_to_segment(p: &Vector3<f32>, start: &Vector3<f32>, end: &Vector3<f32>) -> f32 {
    let segment = end - start;
    let along_segment = (p - start).dot(&segment) / segment.dot(&segment);
    if along_segment < 0. {
        (p - start).norm()
    } else if along_segment < 1. {
        (p - start - segment * along_segment).norm()
    } else {
        (p - end).norm()
    }
}

pub fn distance_to_triangle(
    p: &Vector3<f32>,
    a: &Vector3<f32>,
    b: &Vector3<f32>,
    c: &Vector3<f32>,
    n: &Vector3<f32>,
) -> f32 {
    let ab = a - b;
    let bc = b - c;
    let ca = c - a;

    let sa = n.dot(&bc.cross(&(c - p))) > 0.;
    let sb = n.dot(&ca.cross(&(a - p))) > 0.;
    let sc = n.dot(&ab.cross(&(b - p))) > 0.;

    if sa && sb && sc {
        return (p - a).dot(n).abs();
    }

    let mut distance: f32 = f32::MAX;

    if !sa {
        distance = distance.min(distance_to_segment(p, b, c));
    }
    if !sb {
        distance = distance.min(distance_to_segment(p, c, a));
    }
    if !sc {
        distance = distance.min(distance_to_segment(p, a, b));
    }

    distance
}
