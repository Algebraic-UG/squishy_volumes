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
use squishy_volumes_mesh_util::{angle_weighted_normal, triangle_normal};

use super::*;

fn check(
    mut input_data @ InputData {
        vertex_positions_start,
        vertex_positions_end,
        triangle_indices,
        ..
    }: InputData,
) {
    let vertex_triangle_lists =
        compute_triangle_lists(vertex_positions_start.len(), triangle_indices);
    for step in 0..5 {
        let factor = step as f32 / 5.;

        let frames_per_second = 1;
        input_data.time = factor; // works via fps 1

        println!("factor: {factor}");

        let cpu_vertex_positions: Vec<Vector3<f32>> = vertex_positions_start
            .iter()
            .zip(vertex_positions_end)
            .map(|(start, end)| (1. - factor) * start.xyz() + factor * end.xyz())
            .collect();
        let cpu_triangle_normals = triangle_indices
            .iter()
            .map(|Triangle { a, b, c }| {
                triangle_normal(
                    &cpu_vertex_positions[*a as usize],
                    &cpu_vertex_positions[*b as usize],
                    &cpu_vertex_positions[*c as usize],
                )
            })
            .collect::<Vec<_>>();
        let cpu_vertex_normals = vertex_triangle_lists
            .iter()
            .enumerate()
            .map(|(vertex_index, triangles)| {
                let incident = triangles.iter().map(|index| {
                    let triangle = triangle_indices[*index as usize];
                    let mut others = triangle.iter().filter(|&&i| i != vertex_index as u32);
                    let a = cpu_vertex_positions[*others.next().unwrap() as usize];
                    let b = cpu_vertex_positions[*others.next().unwrap() as usize];
                    (a, b, cpu_triangle_normals[*index as usize])
                });
                angle_weighted_normal(&cpu_vertex_positions[vertex_index], incident)
            })
            .collect::<Vec<_>>();

        let OutputData {
            vertex_positions: gpu_vertex_positions,
            vertex_normals: gpu_vertex_normals,
            triangle_normals: gpu_triangle_normals,
        } = run(
            Settings {
                workgroup_size: 64.try_into().unwrap(),
                dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
                frames_per_second,
            },
            input_data.clone(),
        );

        println!("vertex positions");
        for (cpu, gpu) in cpu_vertex_positions.iter().zip(gpu_vertex_positions) {
            check_iters(cpu.iter(), gpu.iter());
        }
        println!("triangle normals");
        for (cpu, gpu) in cpu_triangle_normals.iter().zip(gpu_triangle_normals) {
            check_iters(cpu.iter(), gpu.iter());
        }
        println!("vertex normals");
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
        time: 0., // hack: this is modified in check
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
        time: 0., // hack: this is modified in check
        vertex_positions_start: &vertices_0,
        vertex_positions_end: &vertices_1,
        triangle_indices: &triangles,
    });
}

#[test]
fn cone() {
    let vertices_0 = cone::vertices();
    let vertices_1 = cone::vertices();
    let triangles = cone::triangles();

    check(InputData {
        time: 0., // hack: this is modified in check
        vertex_positions_start: &vertices_0,
        vertex_positions_end: &vertices_1,
        triangle_indices: &triangles,
    });
}

struct OutputData {
    vertex_positions: Vec<Vector4<f32>>,
    vertex_normals: Vec<Vector4<f32>>,
    triangle_normals: Vec<Vector4<f32>>,
}

fn run(settings: Settings, input_data: InputData) -> OutputData {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), input_data).unwrap();
    let step = AnimateMesh::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        vertex_positions,
        vertex_normals,
        triangle_normals,
    } = step
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
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

    OutputData {
        vertex_positions: vertex_positions.to_vec().unwrap(),
        vertex_normals: vertex_normals.to_vec().unwrap(),
        triangle_normals: triangle_normals.to_vec().unwrap(),
    }
}
