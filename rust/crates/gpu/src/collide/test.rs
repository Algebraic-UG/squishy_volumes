// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use squishy_volumes_util::{
    NORMALIZATION_EPS,
    mesh::{compute_triangle_lists, compute_triangle_opposites},
};

use super::*;

fn check(
    forget_distance: f32,
    accept_distance: f32,
    time_step: f32,
    mut input_data @ InputData {
        particle_positions,
        particle_collider_bits,
        particle_velocities,
        vertex_positions,
        triangle_indices,
        triangle_collider,
        triangle_frictions,
        ..
    }: InputData,
) {
    let vertex_triangle_lists = compute_triangle_lists(vertex_positions.len(), triangle_indices);

    let triangle_normals = triangle_indices
        .iter()
        .map(|Triangle { a, b, c }| {
            let a = vertex_positions[*a as usize].xyz();
            let b = vertex_positions[*b as usize].xyz();
            let c = vertex_positions[*c as usize].xyz();
            (b - a)
                .cross(&(c - a))
                .try_normalize(NORMALIZATION_EPS)
                .unwrap_or(Vector3::zeros())
                .push(0.)
        })
        .collect::<Vec<_>>();
    let vertex_normals = vertex_triangle_lists
        .iter()
        .take(vertex_positions.len())
        .map(|triangles| {
            let mut normal = Vector4::zeros();
            for index in triangles.iter() {
                normal += triangle_normals[*index as usize];
            }
            normal
                .try_normalize(NORMALIZATION_EPS)
                .unwrap_or(Vector4::zeros())
        })
        .collect::<Vec<_>>();
    let triangle_opposites = compute_triangle_opposites(triangle_indices);

    input_data.vertex_normals = &vertex_normals;
    input_data.triangle_normals = &triangle_normals;
    input_data.triangle_opposites = &triangle_opposites;

    let (gpu_particle_collider_bits, gpu_particle_velocites) = run(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            forget_distance,
            accept_distance,
            time_step,
        },
        input_data,
    );

    let mut cpu_particle_collider_bits: Vec<u32> = particle_collider_bits.iter().cloned().collect();
    let mut cpu_particle_velocites: Vec<Vector3<f32>> =
        particle_velocities.iter().map(Vector4::xyz).collect();

    for ((p, bits), velocity) in particle_positions
        .iter()
        .zip(&mut cpu_particle_collider_bits)
        .zip(&mut cpu_particle_velocites)
    {
        // TODO
    }

    println!("collider bits");
    for (cpu, gpu) in cpu_particle_collider_bits
        .into_iter()
        .zip(gpu_particle_collider_bits)
    {
        assert_eq!(cpu, gpu);
    }
    println!("velocites");
    for (cpu, gpu) in cpu_particle_velocites.iter().zip(gpu_particle_velocites) {
        check_iters(cpu.iter(), gpu.iter());
    }
}

#[test]
fn simple() {
    let particle_positions = vec![Vector4::new(1., 1., 1., 0.)];
    let particle_collider_bits = vec![0];
    let particle_velocities = vec![Vector4::zeros()];
    let vertex_positions = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
    ];
    let triangle_indices = vec![Triangle { a: 0, b: 1, c: 2 }];
    let triangle_collider = vec![0];
    let triangle_frictions = vec![0.];

    let forget_distance = 0.13;
    let accept_distance = 0.1;
    let time_step = 0.01;
    let leaf_size = 1.;
    let leaf_threshold = 0;

    check(
        forget_distance,
        accept_distance,
        time_step,
        InputData {
            leaf_size,
            leaf_threshold,
            particle_positions: &particle_positions,
            particle_collider_bits: &particle_collider_bits,
            particle_velocities: &particle_velocities,
            vertex_positions: &vertex_positions,
            triangle_indices: &triangle_indices,
            triangle_collider: &triangle_collider,
            triangle_frictions: &triangle_frictions,

            // hacky: will be computed in check
            vertex_normals: &[],
            triangle_normals: &[],
            triangle_opposites: &[],
        },
    );
}

/*
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
*/

fn run(settings: Settings, input_data: InputData) -> (Vec<u32>, Vec<Vector4<f32>>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), &settings, input_data);
    let step = Collide::new(&context, settings);

    let particle_collider_bits = input.particle_collider_bits.clone();
    let particle_velocities = input.particle_velocities.clone();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output = step
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [particle_collider_bits, particle_velocities]);
    downloads.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [particle_collider_bits, particle_velocities] = downloads.try_into().unwrap();

    (
        particle_collider_bits.to_vec(),
        particle_velocities.to_vec(),
    )
}
