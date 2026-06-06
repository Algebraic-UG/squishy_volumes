// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Rotation3;

use crate::torus;

use super::*;

fn check(
    settings @ Settings {
        cell_size, layers, ..
    }: Settings,
    input_data: InputData,
) {
    let collider_bits_cpu = detect_colliders_on_cpu(cell_size, layers, &input_data);
    let collider_bits_gpu = run(settings, input_data.clone());

    for ((cpu, gpu), id) in collider_bits_cpu
        .into_iter()
        .zip(collider_bits_gpu)
        .zip(input_data.block_ids)
    {
        assert_eq!(cpu, gpu, "{id:?}");
    }
}

#[test]
fn single_triangle() {
    let vertices = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
    ];
    let triangles = vec![Triangle { a: 0, b: 1, c: 2 }];

    let (block_ids, block_table) = build_hash_table_on_cpu_simple(&[Vector4::zeros()]);

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            cell_size: 0.5,
            layers: 3,
        },
        InputData {
            collider_meshes: &[InputDataMesh {
                vertices: &vertices,
                triangles: &triangles,
            }],

            block_ids: &block_ids,
            block_table: &block_table,
        },
    );
}

#[test]
fn specific() {
    let vertices = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
    ];
    let triangles = vec![Triangle { a: 0, b: 1, c: 2 }];

    let (block_ids, block_table) = build_hash_table_on_cpu_simple(&[Vector4::new(4, -1, 0, 0)]);

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            cell_size: 0.5,
            layers: 3,
        },
        InputData {
            collider_meshes: &[InputDataMesh {
                vertices: &vertices,
                triangles: &triangles,
            }],
            block_ids: &block_ids,
            block_table: &block_table,
        },
    );
}

#[test]
fn embedded_triangle() {
    let vertices = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
    ];
    let triangles = vec![Triangle { a: 0, b: 1, c: 2 }];

    let cell_ids: Vec<_> = (-10..=10)
        .flat_map(move |i| {
            (-10..=10).flat_map(move |j| (-10..=10).map(move |k| Vector4::new(i, j, k, 0)))
        })
        .collect();
    let (block_ids, block_table) = build_hash_table_on_cpu_simple(&cell_ids);

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            cell_size: 0.5,
            layers: 3,
        },
        InputData {
            collider_meshes: &[InputDataMesh {
                vertices: &vertices,
                triangles: &triangles,
            }],
            block_ids: &block_ids,
            block_table: &block_table,
        },
    );
}

#[test]
fn two_embedded_triangles() {
    let vertices = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
    ];

    let vertices_2 = vertices
        .iter()
        .map(|v| (Rotation3::from_euler_angles(0.3, 0., 0.) * v.xyz()).push(0.))
        .collect::<Vec<_>>();

    let triangles = vec![Triangle { a: 0, b: 1, c: 2 }];

    let cell_ids: Vec<_> = (-10..=10)
        .flat_map(move |i| {
            (-10..=10).flat_map(move |j| (-10..=10).map(move |k| Vector4::new(i, j, k, 0)))
        })
        .collect();
    let (block_ids, block_table) = build_hash_table_on_cpu_simple(&cell_ids);

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            cell_size: 0.5,
            layers: 3,
        },
        InputData {
            collider_meshes: &[
                InputDataMesh {
                    vertices: &vertices,
                    triangles: &triangles,
                },
                InputDataMesh {
                    vertices: &vertices_2,
                    triangles: &triangles,
                },
            ],

            block_ids: &block_ids,
            block_table: &block_table,
        },
    );
}

#[test]
fn torus() {
    let cell_ids: Vec<_> = (-10..=10)
        .flat_map(move |i| {
            (-10..=10).flat_map(move |j| (-10..=10).map(move |k| Vector4::new(i, j, k, 0)))
        })
        .collect();
    let (block_ids, block_table) = build_hash_table_on_cpu_simple(&cell_ids);

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            cell_size: 0.5,
            layers: 3,
        },
        InputData {
            collider_meshes: &[InputDataMesh {
                vertices: &torus::vertices(),
                triangles: &torus::triangles(),
            }],
            block_ids: &block_ids,
            block_table: &block_table,
        },
    );
}

fn run(settings: Settings, input_data: InputData) -> Vec<u32> {
    let mut context = SHARED_CONTEXT.lock().unwrap();
    let input = Input::new(context.device(), input_data);
    let detect_colliders = DetectColliders::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let Output { collider_bits } = detect_colliders
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, collider_bits);
    download.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
