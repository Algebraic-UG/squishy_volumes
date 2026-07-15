// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use squishy_volumes_mesh_util::{
    DistanceResult, compute_triangle_lists, compute_triangle_opposites, distance_to_triangle,
    segment_distance_result,
};
use squishy_volumes_util::{Aabb, NORMALIZATION_EPS, collider_bits};

use super::*;

fn check(
    forget_distance: f32,
    accept_distance: f32,
    time_step: f32,
    mut input_data @ InputData {
        particle_positions_and_collider_bits,
        particle_velocities,
        vertex_positions,
        triangle_indices,
        triangle_collider,
        triangle_frictions: _, // TODO
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

    let mut cpu_particle_positions_and_collider_bits: Vec<PositionAndColliderBits> =
        particle_positions_and_collider_bits.to_vec();
    let mut cpu_particle_velocites: Vec<Vector3<f32>> =
        particle_velocities.iter().map(Vector4::xyz).collect();

    for (
        PositionAndColliderBits {
            position,
            collider_bits: bits,
        },
        velocity,
    ) in cpu_particle_positions_and_collider_bits
        .iter_mut()
        .zip(&mut cpu_particle_velocites)
    {
        let p = *position;
        let mut closest_triangle_per_collider: [u32; 16] = [u32::MAX; 16];
        let mut min_distance_per_collider: [f32; 16] = [f32::MAX; 16];
        for (triangle_index, ((Triangle { a, b, c }, n), collider)) in triangle_indices
            .iter()
            .zip(&triangle_normals)
            .zip(triangle_collider)
            .enumerate()
        {
            if *n == Vector3::zeros() {
                continue;
            }
            let distance = distance_to_triangle(
                &p.xyz(),
                &vertex_positions[*a as usize].xyz(),
                &vertex_positions[*b as usize].xyz(),
                &vertex_positions[*c as usize].xyz(),
                &n.xyz(),
            );
            if distance < forget_distance
                && distance < min_distance_per_collider[*collider as usize]
            {
                min_distance_per_collider[*collider as usize] = distance;
                closest_triangle_per_collider[*collider as usize] = triangle_index as u32;
            }
        }

        for (collider, closest_triangle) in closest_triangle_per_collider.into_iter().enumerate() {
            if closest_triangle == u32::MAX {
                collider_bits::set(bits, collider, None);
                continue;
            }

            let triangle = triangle_indices[closest_triangle as usize];

            let opps = triangle_opposites[closest_triangle as usize];
            let n = triangle_normals[closest_triangle as usize].xyz();
            let a = vertex_positions[triangle.a as usize].xyz();
            let b = vertex_positions[triangle.b as usize].xyz();
            let c = vertex_positions[triangle.c as usize].xyz();
            let a_n = vertex_normals[triangle.a as usize].xyz();
            let b_n = vertex_normals[triangle.b as usize].xyz();
            let c_n = vertex_normals[triangle.c as usize].xyz();
            let ab_n = if opps.ab != u32::MAX {
                triangle_normals[opps.ab as usize].xyz()
            } else {
                Vector3::zeros()
            };
            let bc_n = if opps.bc != u32::MAX {
                triangle_normals[opps.bc as usize].xyz()
            } else {
                Vector3::zeros()
            };
            let ca_n = if opps.ca != u32::MAX {
                triangle_normals[opps.ca as usize].xyz()
            } else {
                Vector3::zeros()
            };

            let ab = a - b;
            let bc = b - c;
            let ca = c - a;

            let sa = n.dot(&bc.cross(&(c - p))) > 0.;
            let sb = n.dot(&ca.cross(&(a - p))) > 0.;
            let sc = n.dot(&ab.cross(&(b - p))) > 0.;

            let DistanceResult {
                distance,
                to_p,
                normal,
            } = if sa && sb && sc {
                DistanceResult {
                    distance: (p - a).dot(&n).abs(),
                    to_p: n * (p - a).dot(&n),
                    normal: n,
                }
            } else {
                [
                    segment_distance_result(&p, &a, &b, &a_n, &ab_n, &b_n),
                    segment_distance_result(&p, &b, &c, &b_n, &bc_n, &c_n),
                    segment_distance_result(&p, &c, &a, &c_n, &ca_n, &a_n),
                ]
                .into_iter()
                .min_by(|a, b| a.distance.total_cmp(&b.distance))
                .unwrap()
            };

            if normal == Vector3::zeros() {
                collider_bits::set(bits, collider, None);
                continue;
            }

            let new_side = 0. <= to_p.dot(&normal);
            let Some(prior_side) = collider_bits::get(*bits, collider) else {
                if distance < accept_distance {
                    collider_bits::set(bits, collider, Some(new_side));
                }
                continue;
            };

            if prior_side == new_side {
                continue;
            }

            *velocity -= to_p / time_step;
        }
    }

    println!("collider bits");
    for (particle_index, (cpu, gpu)) in cpu_particle_positions_and_collider_bits
        .iter()
        .zip(gpu_particle_collider_bits)
        .enumerate()
    {
        println!("{particle_index} {:?}", cpu.position);
        let cpu = cpu.collider_bits;
        let gpu = gpu.collider_bits;

        assert_eq!(
            cpu & 0xFFFF_0000,
            gpu & 0xFFFF_0000,
            "{particle_index}: {cpu:032b} vs {gpu:032b}"
        );

        let mask = cpu >> 16;

        assert_eq!(
            cpu & mask,
            gpu & mask,
            "{particle_index}: {cpu:032b} vs {gpu:032b}"
        );
    }
    println!("velocites");
    for (particle_index, (cpu, gpu)) in cpu_particle_velocites
        .iter()
        .zip(gpu_particle_velocites)
        .enumerate()
    {
        println!(
            "{particle_index} {:?} {:032b}",
            cpu_particle_positions_and_collider_bits[particle_index].position,
            cpu_particle_positions_and_collider_bits[particle_index].collider_bits,
        );
        check_iters(cpu.iter(), gpu.iter());
    }
}

#[test]
fn simple() {
    let particle_positions_and_collider_bits = [PositionAndColliderBits {
        position: Vector3::new(0.5, 0.5, 0.5),
        collider_bits: 0x0001_0000,
    }];
    let particle_velocities = vec![Vector4::zeros()];
    let vertex_positions = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
    ];
    let triangle_indices = vec![Triangle { a: 0, b: 1, c: 2 }];
    let triangle_collider = vec![0];
    let triangle_frictions = vec![0.];

    let forget_distance = 2.;
    let accept_distance = 1.;
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
            particle_positions_and_collider_bits: &particle_positions_and_collider_bits,
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

#[test]
fn simple2() {
    let particle_positions_and_collider_bits = [
        PositionAndColliderBits {
            position: Vector3::new(0.5, 0.5, 0.5),
            collider_bits: 0x0001_0000,
        },
        PositionAndColliderBits {
            position: Vector3::new(1., 0., 0.5),
            collider_bits: 0x0001_0000,
        },
        PositionAndColliderBits {
            position: Vector3::new(1., 1., 1.5),
            collider_bits: 0x0001_0000,
        },
    ];
    let particle_velocities = vec![Vector4::zeros(); particle_positions_and_collider_bits.len()];
    let vertex_positions = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
        Vector4::new(2., 1., 0., 0.),
    ];
    let triangle_indices = vec![
        Triangle { a: 0, b: 1, c: 2 },
        Triangle { a: 0, b: 2, c: 3 },
        Triangle { a: 0, b: 3, c: 1 },
    ];
    let triangle_collider = vec![0; triangle_indices.len()];
    let triangle_frictions = vec![0.; triangle_indices.len()];

    let forget_distance = 2.;
    let accept_distance = 1.;
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
            particle_positions_and_collider_bits: &particle_positions_and_collider_bits,
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

#[test]
fn torus() {
    let vertex_positions = torus::vertices();
    let triangle_indices = torus::triangles();

    let aabb = Aabb::new(vertex_positions.iter().map(Vector4::xyz));
    let particle_positions_and_collider_bits: Vec<_> = aabb
        .lattice(0.2)
        .1
        .map(|position| PositionAndColliderBits {
            position,
            collider_bits: 0,
        })
        .collect();
    let particle_velocities = vec![Vector4::zeros(); particle_positions_and_collider_bits.len()];
    let triangle_collider: Vec<u32> = vec![0; triangle_indices.len()];
    let triangle_frictions = vec![0.; triangle_indices.len()];

    let forget_distance = 0.7;
    let accept_distance = 0.5;
    let time_step = 0.01;
    let leaf_size = 0.5;
    let leaf_threshold = 0;

    check(
        forget_distance,
        accept_distance,
        time_step,
        InputData {
            leaf_size,
            leaf_threshold,
            particle_positions_and_collider_bits: &particle_positions_and_collider_bits,
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

fn run(
    settings: Settings,
    input_data: InputData,
) -> (Vec<PositionAndColliderBits>, Vec<Vector4<f32>>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), &settings, input_data).unwrap();
    let step = Collide::new(&mut context, settings).unwrap();

    let particle_positions_and_collider_bits = input.particle_positions_and_collider_bits.clone();
    let particle_velocities = input.particle_velocities.clone();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output = step
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(
        &context,
        [particle_positions_and_collider_bits, particle_velocities],
    );
    downloads.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [particle_positions_and_collider_bits, particle_velocities] = downloads.try_into().unwrap();

    (
        particle_positions_and_collider_bits.to_vec().unwrap(),
        particle_velocities.to_vec().unwrap(),
    )
}
