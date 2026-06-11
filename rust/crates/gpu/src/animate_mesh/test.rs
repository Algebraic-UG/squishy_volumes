// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use squishy_volumes_util::NORMALIZATION_EPS;

use super::*;

fn check(
    input_data @ InputData {
        vertex_positions_start,
        vertex_positions_end,
        triangle_indices,
    }: InputData,
) {
    let vertex_triangle_lists = triangle_lists(vertex_positions_start.len(), triangle_indices);
    for step in 0..5 {
        let factor = step as f32 / 5.;

        let cpu_vertex_positions: Vec<Vector3<f32>> = vertex_positions_start
            .iter()
            .zip(vertex_positions_end)
            .map(|(start, end)| (1. - factor) * start.xyz() + factor * end.xyz())
            .collect();
        let cpu_triangle_normals = triangle_indices
            .iter()
            .map(|Triangle { a, b, c }| {
                let a = cpu_vertex_positions[*a as usize];
                let b = cpu_vertex_positions[*b as usize];
                let c = cpu_vertex_positions[*c as usize];
                (b - a)
                    .cross(&(c - a))
                    .try_normalize(NORMALIZATION_EPS)
                    .unwrap_or(Vector3::zeros())
            })
            .collect::<Vec<_>>();
        let cpu_vertex_normals = vertex_triangle_lists
            .iter()
            .map(|triangles| {
                let mut normal = Vector3::zeros();
                for index in triangles.iter() {
                    normal += cpu_triangle_normals[*index as usize];
                }
                normal
            })
            .collect::<Vec<_>>();

        let (gpu_vertex_positions, gpu_vertex_normals, gpu_triangle_normals) = run(
            Settings {
                workgroup_size: 64.try_into().unwrap(),
                dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            },
            input_data.clone(),
            Parameters { factor },
        );

        for (cpu, gpu) in cpu_vertex_positions.iter().zip(gpu_vertex_positions) {
            check_iters(cpu.iter(), gpu.iter());
        }
        for (cpu, gpu) in cpu_triangle_normals.iter().zip(gpu_triangle_normals) {
            check_iters(cpu.iter(), gpu.iter());
        }
        for (cpu, gpu) in cpu_vertex_normals.iter().zip(gpu_vertex_normals) {
            check_iters(cpu.iter(), gpu.iter());
        }
    }
}

#[test]
fn simple() {
    let vertices_0 = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
    ];
    let vertices_1 = vec![
        -Vector4::new(1., 1., 1., 0.),
        -Vector4::new(0., 1., 0., 0.),
        -Vector4::new(1., 0., 0., 0.),
    ];
    let triangles = vec![Triangle { a: 0, b: 1, c: 2 }];

    check(InputData {
        vertex_positions_start: &vertices_0,
        vertex_positions_end: &vertices_1,
        triangle_indices: &triangles,
    });
}

#[test]
fn torus() {
    let vertices_0 = torus::vertices();
    let vertices_1 = vertices_0.iter().map(|v| v * 2.).collect::<Vec<_>>();
    let triangles = torus::triangles();

    check(InputData {
        vertex_positions_start: &vertices_0,
        vertex_positions_end: &vertices_1,
        triangle_indices: &triangles,
    });
}

fn run(
    settings: Settings,
    input_data: InputData,
    parameters: Parameters,
) -> (Vec<Vector4<f32>>, Vec<Vector4<f32>>, Vec<Vector4<f32>>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), input_data);
    let step = AnimateMesh::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        vertex_positions,
        vertex_normals,
        triangle_normals,
    } = step
        .record(&mut context, &mut (&mut encoder).into(), input, parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(
        &context,
        [vertex_positions, vertex_normals, triangle_normals],
    );
    downloads.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [vertex_positions, vertex_normals, triangle_normals] = downloads.try_into().unwrap();

    (
        vertex_positions.to_vec(),
        vertex_normals.to_vec(),
        triangle_normals.to_vec(),
    )
}
