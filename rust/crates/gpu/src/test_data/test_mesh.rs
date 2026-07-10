// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Vector3, Vector4};
use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};
use squishy_volumes_mesh_util::{
    Opposites, Triangle, compute_triangle_lists, compute_triangle_opposites,
};
use squishy_volumes_util::{Aabb, NORMALIZATION_EPS};

pub struct TestMesh {
    pub vertex_positions_a: Vec<Vector4<f32>>,
    pub vertex_positions_b: Vec<Vector4<f32>>,
    pub triangle_frictions_a: Vec<f32>,
    pub triangle_frictions_b: Vec<f32>,
    pub triangle_indices: Vec<Triangle>,
    pub triangle_opposites: Vec<Opposites>,

    pub triangle_normals_a: Vec<Vector4<f32>>,
    pub vertex_normals_a: Vec<Vector4<f32>>,
}

pub struct TestMeshNormals {}

impl TestMesh {
    pub fn new(num_triangles: usize, aabb: Aabb<Vector3<f32>>) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(33);
        let vertex_positions_a: Vec<_> = (0..num_triangles * 3)
            .flat_map(|_| {
                let a = Vector4::new(
                    rng.random_range(aabb.min.x..aabb.max.x),
                    rng.random_range(aabb.min.y..aabb.max.y),
                    rng.random_range(aabb.min.z..aabb.max.z),
                    0.,
                );
                let b = a + Vector4::new(
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                    0.,
                );
                let c = a + Vector4::new(
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                    0.,
                );
                [a, b, c]
            })
            .collect();
        let vertex_positions_b = vertex_positions_a
            .iter()
            .map(|v| {
                v + Vector4::new(
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                    0.,
                )
            })
            .collect();
        let triangle_frictions_a = (0..num_triangles)
            .map(|_| rng.random_range(0.0..100.0))
            .collect();
        let triangle_frictions_b = (0..num_triangles)
            .map(|_| rng.random_range(0.0..100.0))
            .collect();
        let triangle_indices: Vec<_> = (0..num_triangles)
            .map(|i| Triangle {
                a: i as u32 * 3,
                b: i as u32 * 3 + 1,
                c: i as u32 * 3 + 2,
            })
            .collect();

        let vertex_triangle_lists =
            compute_triangle_lists(vertex_positions_a.len(), &triangle_indices);
        let triangle_opposites = compute_triangle_opposites(&triangle_indices);

        let triangle_normals_a = triangle_indices
            .iter()
            .map(|Triangle { a, b, c }| {
                let a = vertex_positions_a[*a as usize].xyz();
                let b = vertex_positions_a[*b as usize].xyz();
                let c = vertex_positions_a[*c as usize].xyz();
                (b - a)
                    .cross(&(c - a))
                    .try_normalize(NORMALIZATION_EPS)
                    .unwrap_or(Vector3::zeros())
                    .push(0.)
            })
            .collect::<Vec<_>>();
        let vertex_normals_a = vertex_triangle_lists
            .iter()
            .take(vertex_positions_a.len())
            .map(|triangles| {
                let mut normal = Vector4::zeros();
                for index in triangles.iter() {
                    normal += triangle_normals_a[*index as usize];
                }
                normal
                    .try_normalize(NORMALIZATION_EPS)
                    .unwrap_or(Vector4::zeros())
            })
            .collect::<Vec<_>>();

        Self {
            vertex_positions_a,
            vertex_positions_b,
            triangle_frictions_a,
            triangle_frictions_b,
            triangle_indices,
            triangle_opposites,
            triangle_normals_a,
            vertex_normals_a,
        }
    }
}
