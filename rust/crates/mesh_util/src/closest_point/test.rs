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
use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};

use super::*;
use crate::{Triangle, compute_triangle_lists, compute_triangle_opposites, distance_to_triangle};

fn random_point(rng: &mut ChaCha8Rng, extent: f32) -> Vector3<f32> {
    Vector3::from_fn(|_, _| rng.random_range(-extent..extent))
}

fn test_triangles() -> Vec<[Vector3<f32>; 3]> {
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let obtuse = [
        Vector3::new(0., 0., 0.),
        Vector3::new(1., 0., 0.),
        Vector3::new(0.5, 0.05, 0.),
    ];
    let needle = [
        Vector3::new(0., 0., 0.),
        Vector3::new(1., 0., 0.),
        Vector3::new(1., 0.02, 0.3),
    ];
    let mut triangles = vec![obtuse, needle];
    while triangles.len() < 20 {
        let triangle = [(); 3].map(|_| random_point(&mut rng, 1.));
        if triangle_normal(&triangle[0], &triangle[1], &triangle[2]) != Vector3::zeros() {
            triangles.push(triangle);
        }
    }
    triangles
}

#[test]
fn barycentric_identities() {
    let mut rng = ChaCha8Rng::seed_from_u64(1);
    for [a, b, c] in test_triangles() {
        let n = triangle_normal(&a, &b, &c);
        for (vertex, expected) in [(a, Vector3::x()), (b, Vector3::y()), (c, Vector3::z())] {
            let barycentric = barycentric_coordinates(&vertex, [&a, &b, &c], &n);
            assert!((barycentric - expected).norm() < 1e-4, "{barycentric}");
        }
        for _ in 0..100 {
            let p = random_point(&mut rng, 2.);
            let barycentric = barycentric_coordinates(&p, [&a, &b, &c], &n);
            assert!((barycentric.sum() - 1.).abs() < 1e-3);
            let projection = p - n * (p - a).dot(&n);
            let reconstructed = a * barycentric.x + b * barycentric.y + c * barycentric.z;
            assert!((reconstructed - projection).norm() < 1e-3 * (1. + p.norm()));
        }
    }
}

fn brute_force_distance(p: &Vector3<f32>, [a, b, c]: &[Vector3<f32>; 3]) -> f32 {
    const N: usize = 100;
    let mut distance = f32::MAX;
    for i in 0..=N {
        for j in 0..=N - i {
            let (u, v) = (i as f32 / N as f32, j as f32 / N as f32);
            distance = distance.min((a + (b - a) * u + (c - a) * v - p).norm());
        }
    }
    distance
}

#[test]
fn distance_matches_brute_force() {
    let mut rng = ChaCha8Rng::seed_from_u64(2);
    for triangle @ [a, b, c] in test_triangles() {
        let n = triangle_normal(&a, &b, &c);
        let resolution = [(b - a).norm(), (c - a).norm()].into_iter().sum::<f32>() / 100.;
        for _ in 0..20 {
            let p = random_point(&mut rng, 1.5);
            let distance = distance_to_triangle(&p, &a, &b, &c, &n);
            let brute_force = brute_force_distance(&p, &triangle);
            assert!(
                distance <= brute_force + 1e-5,
                "{p}: {distance} vs {brute_force}"
            );
            assert!(
                brute_force <= distance + resolution,
                "{p}: {distance} vs {brute_force}"
            );
        }
    }
}

#[test]
fn closest_feature_is_on_triangle() {
    let mut rng = ChaCha8Rng::seed_from_u64(3);
    let normals = FeatureNormals {
        vertices: [Vector3::x(); 3],
        edges: [Vector3::x(); 3],
    };
    for [a, b, c] in test_triangles() {
        let n = triangle_normal(&a, &b, &c);
        for _ in 0..100 {
            let p = random_point(&mut rng, 1.5);
            let result = closest_feature(&p, [&a, &b, &c], &n, &normals);
            let expected = distance_to_triangle(&p, &a, &b, &c, &n);
            assert!((result.distance - expected).abs() < 1e-5);
            assert!((result.to_p.norm() - result.distance).abs() < 1e-5);
            let closest = p - result.to_p;
            assert!((closest - a).dot(&n).abs() < 1e-5);
            let barycentric = barycentric_coordinates(&closest, [&a, &b, &c], &n);
            assert!(barycentric.iter().all(|x| (-1e-4..=1. + 1e-4).contains(x)));
        }
    }
}

struct Mesh {
    vertices: Vec<Vector3<f32>>,
    triangles: Vec<Triangle>,
}

fn mesh(vertices: Vec<[f32; 3]>, triangles: &[[u32; 3]]) -> Mesh {
    Mesh {
        vertices: vertices.into_iter().map(Vector3::from).collect(),
        triangles: triangles.iter().map(|t| t.iter().cloned().into()).collect(),
    }
}

fn test_meshes() -> Vec<Mesh> {
    let tetrahedron = mesh(
        vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
    );
    let cube = mesh(
        (0..8)
            .map(|i| [(i & 1) as f32, ((i >> 1) & 1) as f32, ((i >> 2) & 1) as f32])
            .collect(),
        &[
            [0, 2, 1],
            [1, 2, 3],
            [4, 5, 6],
            [5, 7, 6],
            [0, 1, 4],
            [1, 5, 4],
            [2, 6, 3],
            [3, 6, 7],
            [0, 4, 2],
            [2, 4, 6],
            [1, 3, 5],
            [3, 7, 5],
        ],
    );
    let dented_octahedron = mesh(
        vec![
            [1., 0., 0.],
            [-1., 0., 0.],
            [0., 1., 0.],
            [0., -1., 0.],
            [0., 0., -0.3],
            [0., 0., -1.],
        ],
        &[
            [0, 2, 4],
            [2, 1, 4],
            [1, 3, 4],
            [3, 0, 4],
            [2, 0, 5],
            [1, 2, 5],
            [3, 1, 5],
            [0, 3, 5],
        ],
    );
    vec![tetrahedron, cube, dented_octahedron]
}

fn winding_number(p: &Vector3<f32>, mesh: &Mesh) -> f32 {
    let solid_angles: f32 = mesh
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = [t.a, t.b, t.c].map(|i| mesh.vertices[i as usize] - p);
            let (la, lb, lc) = (a.norm(), b.norm(), c.norm());
            let numerator = a.dot(&b.cross(&c));
            let denominator = la * lb * lc + a.dot(&b) * lc + b.dot(&c) * la + c.dot(&a) * lb;
            2. * numerator.atan2(denominator)
        })
        .sum();
    solid_angles / (4. * std::f32::consts::PI)
}

fn pseudo_normal_inside(p: &Vector3<f32>, mesh: &Mesh) -> bool {
    let vertices = &mesh.vertices;
    let triangle_normals: Vec<_> = mesh
        .triangles
        .iter()
        .map(|t| {
            triangle_normal(
                &vertices[t.a as usize],
                &vertices[t.b as usize],
                &vertices[t.c as usize],
            )
        })
        .collect();
    let vertex_normals: Vec<_> = compute_triangle_lists(vertices.len(), &mesh.triangles)
        .iter()
        .enumerate()
        .map(|(vertex_index, triangles)| {
            let incident = triangles.iter().map(|&triangle_index| {
                let triangle = mesh.triangles[triangle_index as usize];
                let mut others = triangle.iter().filter(|&&i| i != vertex_index as u32);
                let a = vertices[*others.next().unwrap() as usize];
                let b = vertices[*others.next().unwrap() as usize];
                (a, b, triangle_normals[triangle_index as usize])
            });
            angle_weighted_normal(&vertices[vertex_index], incident)
        })
        .collect();
    let opposites = compute_triangle_opposites(&mesh.triangles);

    let (closest, _) = mesh
        .triangles
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let [a, b, c] = [t.a, t.b, t.c].map(|i| &vertices[i as usize]);
            (i, distance_to_triangle(p, a, b, c, &triangle_normals[i]))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap();

    let t = mesh.triangles[closest];
    let n = triangle_normals[closest];
    let edge_normal = |opposite: u32| {
        let neighbour_normal = (opposite != u32::MAX).then(|| &triangle_normals[opposite as usize]);
        edge_pseudo_normal(&n, neighbour_normal)
    };
    let o = opposites[closest];
    let result = closest_feature(
        p,
        [t.a, t.b, t.c].map(|i| &vertices[i as usize]),
        &n,
        &FeatureNormals {
            vertices: [t.a, t.b, t.c].map(|i| vertex_normals[i as usize]),
            edges: [edge_normal(o.ab), edge_normal(o.bc), edge_normal(o.ca)],
        },
    );
    result.to_p.dot(&result.normal) < 0.
}

#[test]
fn pseudo_normals_classify_inside() {
    let mut rng = ChaCha8Rng::seed_from_u64(4);
    for mesh in test_meshes() {
        let random_points: Vec<_> = (0..2000).map(|_| random_point(&mut rng, 1.5)).collect();
        let points_near_corners: Vec<_> = mesh
            .vertices
            .iter()
            .flat_map(|vertex| [*vertex; 200])
            .map(|vertex| vertex + random_point(&mut rng, 0.1))
            .collect();
        for p in random_points.into_iter().chain(points_near_corners) {
            let winding = winding_number(&p, &mesh);
            let closed_mesh_winding = winding.round();
            assert!(
                (winding - closed_mesh_winding).abs() < 1e-3
                    && (0. ..=1.).contains(&closed_mesh_winding),
                "{p}: winding number {winding} of a closed mesh"
            );
            assert_eq!(
                pseudo_normal_inside(&p, &mesh),
                closed_mesh_winding == 1.,
                "{p}"
            );
        }
    }
}
