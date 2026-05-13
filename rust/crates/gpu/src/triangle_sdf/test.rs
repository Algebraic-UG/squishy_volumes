// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use squishy_volumes_util::{Aabb, NORMALIZATION_EPS};

use super::*;

fn check(
    input_data @ InputData {
        test_positions,
        vertices,
        triangles,
    }: InputData,
) {
    let cpu: Vec<f32> = test_positions
        .iter()
        .map(Vector4::xyz)
        .map(|p| {
            triangles
                .iter()
                .map(|triangle| {
                    let a = &vertices[triangle.a as usize].xyz();
                    let b = &vertices[triangle.b as usize].xyz();
                    let c = &vertices[triangle.c as usize].xyz();
                    let ab = a - b;
                    let bc = b - c;
                    let ca = c - a;
                    let normal_area_2 = (-ab).cross(&ca);
                    let area_2 = normal_area_2.norm();
                    if area_2 < NORMALIZATION_EPS {
                        return f32::MAX;
                    }

                    let normal = normal_area_2 / area_2;

                    let bary_c = (p - b).dot(&normal.cross(&ab)) / area_2;
                    let bary_a = (p - c).dot(&normal.cross(&bc)) / area_2;
                    let bary_b = (p - a).dot(&normal.cross(&ca)) / area_2;

                    let sa = bary_a < 0.;
                    let sb = bary_b < 0.;
                    let sc = bary_c < 0.;

                    if (sa && sb && sc) || (!sa && !sb && !sc) {
                        (p - a).dot(&normal).abs()
                    } else {
                        let edge_distance = |start: &Vector3<f32>, end: &Vector3<f32>| {
                            let segment = end - start;
                            let along_segment = (p - start).dot(&segment) / segment.norm_squared();

                            if along_segment < 0. {
                                p - start
                            } else if along_segment < 1. {
                                p - start - segment * along_segment
                            } else {
                                p - end
                            }
                            .norm()
                        };

                        [
                            edge_distance(a, b),
                            edge_distance(b, c),
                            edge_distance(c, a),
                        ]
                        .into_iter()
                        .min_by(f32::total_cmp)
                        .unwrap()
                    }
                })
                .min_by(f32::total_cmp)
                .unwrap()
        })
        .collect();
    let gpu = run(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
        },
        input_data,
    );

    check_iters(cpu.iter(), gpu.iter());
}

#[test]
fn test_simple() {
    let positions = vec![
        Vector4::new(0., 0., 0., 0.),  // edge
        Vector4::new(1., 1., 0., 0.),  //face
        Vector4::new(-1., 1., 0., 0.), // corner
    ];
    let vertices = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
    ];
    let triangles = vec![Triangle { a: 0, b: 1, c: 2 }];

    check(InputData {
        test_positions: &positions,
        vertices: &vertices,
        triangles: &triangles,
    });
}

#[test]
fn test_degenerate() {
    let positions = vec![Vector4::zeros()];
    let vertices = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
    ];
    let triangles = vec![Triangle { a: 0, b: 1, c: 1 }];

    check(InputData {
        test_positions: &positions,
        vertices: &vertices,
        triangles: &triangles,
    });
}

#[test]
fn test_torus() {
    let vertices = torus::vertices();
    let triangles = torus::triangles();
    let aabb = Aabb::new(vertices.iter().map(Vector4::xyz));
    let test_positions: Vec<_> = aabb.lattice(0.5).1.map(|v| v.push(0.)).collect();

    check(InputData {
        test_positions: &test_positions,
        vertices: &vertices,
        triangles: &triangles,
    });
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;
    let mut rng = ChaCha8Rng::seed_from_u64(33);

    let test_positions = random_vecs(&mut rng, 100);
    let vertices = random_vecs(&mut rng, 100);
    let indices: Vec<u32> = rng
        .random_iter::<u32>()
        .take(50 * 3)
        .map(|i| i % vertices.len() as u32)
        .collect();
    let triangles: Vec<_> = indices
        .chunks_exact(3)
        .map(|chunk| Triangle {
            a: chunk[0],
            b: chunk[1],
            c: chunk[2],
        })
        .collect();
    check(InputData {
        test_positions: &test_positions,
        vertices: &vertices,
        triangles: &triangles,
    });
}

fn run(settings: Settings, input_data: InputData) -> Vec<f32> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), input_data);
    let step = TriangleSdf::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { sdf } = step
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, sdf);
    download.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
