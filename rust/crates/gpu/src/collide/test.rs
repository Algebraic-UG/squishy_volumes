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
use squishy_volumes_mesh_util::{
    DistanceResult, FeatureNormals, angle_weighted_normal, barycentric_coordinates,
    closest_feature, compute_triangle_lists, compute_triangle_opposites, distance_to_triangle,
    edge_pseudo_normal, triangle_normal,
};
use squishy_volumes_util::{Aabb, Contact, NORMALIZATION_EPS, collider_bits, collision_response};

use super::*;

fn check(
    forget_distance: f32,
    accept_distance: f32,
    mut input_data @ InputData {
        time_step,
        particle_positions_and_collider_bits,
        particle_velocities,
        vertex_positions,
        vertex_velocities,
        triangle_indices,
        triangle_collider,
        triangle_frictions,
        triangle_dampings,
        ..
    }: InputData,
    check_collider_bits: bool,
) {
    let vertex_triangle_lists = compute_triangle_lists(vertex_positions.len(), triangle_indices);

    let triangle_normals = triangle_indices
        .iter()
        .map(|Triangle { a, b, c }| {
            triangle_normal(
                &vertex_positions[*a as usize].xyz(),
                &vertex_positions[*b as usize].xyz(),
                &vertex_positions[*c as usize].xyz(),
            )
            .push(0.)
        })
        .collect::<Vec<_>>();
    let vertex_normals = vertex_triangle_lists
        .iter()
        .take(vertex_positions.len())
        .enumerate()
        .map(|(vertex_index, triangles)| {
            let incident = triangles.iter().map(|index| {
                let triangle = triangle_indices[*index as usize];
                let mut others = triangle.iter().filter(|&&i| i != vertex_index as u32);
                let a = vertex_positions[*others.next().unwrap() as usize].xyz();
                let b = vertex_positions[*others.next().unwrap() as usize].xyz();
                (a, b, triangle_normals[*index as usize].xyz())
            });
            angle_weighted_normal(&vertex_positions[vertex_index].xyz(), incident).push(0.)
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
            let edge_normal = |opposite: u32| {
                let neighbour_normal =
                    (opposite != u32::MAX).then(|| triangle_normals[opposite as usize].xyz());
                edge_pseudo_normal(&n, neighbour_normal.as_ref())
            };
            let DistanceResult {
                distance,
                to_p,
                normal,
            } = closest_feature(
                &p,
                [&a, &b, &c],
                &n,
                &FeatureNormals {
                    vertices: [triangle.a, triangle.b, triangle.c]
                        .map(|vertex| vertex_normals[vertex as usize].xyz()),
                    edges: [opps.ab, opps.bc, opps.ca].map(edge_normal),
                },
            );

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

            if distance < NORMALIZATION_EPS {
                continue;
            }

            let barycentric = barycentric_coordinates(&p, [&a, &b, &c], &n);
            let corner_velocity = |vertex: u32| vertex_velocities[vertex as usize].xyz();
            let contact = Contact {
                to_p,
                collider_velocity: corner_velocity(triangle.a) * barycentric.x
                    + corner_velocity(triangle.b) * barycentric.y
                    + corner_velocity(triangle.c) * barycentric.z,
                friction: triangle_frictions[closest_triangle as usize],
                damping: triangle_dampings[closest_triangle as usize],
            };
            *velocity = collision_response(velocity, &contact, time_step);
        }
    }

    if check_collider_bits {
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
    let particle_flags = vec![Default::default()];
    let particle_velocities = vec![Vector4::zeros()];
    let vertex_positions = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
    ];
    let triangle_indices = vec![Triangle { a: 0, b: 1, c: 2 }];
    let triangle_collider = vec![0];
    let triangle_frictions = vec![0.];
    let triangle_dampings = vec![0.];

    let forget_distance = 2.;
    let accept_distance = 1.;
    let time_step = 0.01;
    let leaf_size = 1.;
    let leaf_threshold = 0;

    check(
        forget_distance,
        accept_distance,
        InputData {
            time_step,
            leaf_size,
            leaf_threshold,
            particle_flags: &particle_flags,
            particle_positions_and_collider_bits: &particle_positions_and_collider_bits,
            particle_velocities: &particle_velocities,
            vertex_positions: &vertex_positions,
            vertex_velocities: &vec![Vector4::zeros(); vertex_positions.len()],
            triangle_indices: &triangle_indices,
            triangle_collider: &triangle_collider,
            triangle_frictions: &triangle_frictions,
            triangle_dampings: &triangle_dampings,

            // hacky: will be computed in check
            vertex_normals: &[],
            triangle_normals: &[],
            triangle_opposites: &[],
        },
        true,
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
    let particle_flags = vec![Default::default(); particle_positions_and_collider_bits.len()];
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
    let triangle_dampings = vec![0.; triangle_indices.len()];

    let forget_distance = 2.;
    let accept_distance = 1.;
    let time_step = 0.01;
    let leaf_size = 1.;
    let leaf_threshold = 0;

    check(
        forget_distance,
        accept_distance,
        InputData {
            time_step,
            leaf_size,
            leaf_threshold,
            particle_flags: &particle_flags,
            particle_positions_and_collider_bits: &particle_positions_and_collider_bits,
            particle_velocities: &particle_velocities,
            vertex_positions: &vertex_positions,
            vertex_velocities: &vec![Vector4::zeros(); vertex_positions.len()],
            triangle_indices: &triangle_indices,
            triangle_collider: &triangle_collider,
            triangle_frictions: &triangle_frictions,
            triangle_dampings: &triangle_dampings,

            // hacky: will be computed in check
            vertex_normals: &[],
            triangle_normals: &[],
            triangle_opposites: &[],
        },
        true,
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
    let particle_flags = vec![Default::default(); particle_positions_and_collider_bits.len()];
    let particle_velocities = vec![Vector4::zeros(); particle_positions_and_collider_bits.len()];
    let triangle_collider: Vec<u32> = vec![0; triangle_indices.len()];
    let triangle_frictions = vec![0.; triangle_indices.len()];
    let triangle_dampings = vec![0.; triangle_indices.len()];

    let forget_distance = 0.7;
    let accept_distance = 0.5;
    let time_step = 0.01;
    let leaf_size = 0.5;
    let leaf_threshold = 0;

    check(
        forget_distance,
        accept_distance,
        InputData {
            time_step,
            leaf_size,
            leaf_threshold,
            particle_flags: &particle_flags,
            particle_positions_and_collider_bits: &particle_positions_and_collider_bits,
            particle_velocities: &particle_velocities,
            vertex_positions: &vertex_positions,
            vertex_velocities: &vec![Vector4::zeros(); vertex_positions.len()],
            triangle_indices: &triangle_indices,
            triangle_collider: &triangle_collider,
            triangle_frictions: &triangle_frictions,
            triangle_dampings: &triangle_dampings,

            // hacky: will be computed in check
            vertex_normals: &[],
            triangle_normals: &[],
            triangle_opposites: &[],
        },
        false,
    );
}

#[test]
fn dented_octahedron_with_concave_edges_and_corner() {
    let vertex_positions = [
        [1., 0., 0.],
        [-1., 0., 0.],
        [0., 1., 0.],
        [0., -1., 0.],
        [0., 0., -0.3],
        [0., 0., -1.],
    ]
    .map(|[x, y, z]| Vector4::new(x, y, z, 0.))
    .to_vec();
    let triangle_indices = [
        [0, 2, 4],
        [2, 1, 4],
        [1, 3, 4],
        [3, 0, 4],
        [2, 0, 5],
        [1, 2, 5],
        [3, 1, 5],
        [0, 3, 5],
    ]
    .map(|[a, b, c]| Triangle { a, b, c })
    .to_vec();

    let off_the_symmetry_planes = Vector3::new(0.013, 0.007, 0.011);
    let aabb = Aabb::new(vertex_positions.iter().map(Vector4::xyz));
    let particle_positions_and_collider_bits: Vec<_> = aabb
        .lattice(0.1)
        .1
        .map(|position| PositionAndColliderBits {
            position: position + off_the_symmetry_planes,
            collider_bits: 0,
        })
        .collect();
    let particle_flags = vec![Default::default(); particle_positions_and_collider_bits.len()];
    let towards_the_center: Vec<_> = particle_positions_and_collider_bits
        .iter()
        .map(|particle| (-particle.position).push(0.))
        .collect();
    let triangle_collider: Vec<u32> = vec![0; triangle_indices.len()];
    let triangle_frictions = vec![0.; triangle_indices.len()];
    let triangle_dampings = vec![0.; triangle_indices.len()];

    check(
        0.7,
        0.5,
        InputData {
            time_step: 0.01,
            leaf_size: 0.5,
            leaf_threshold: 0,
            particle_flags: &particle_flags,
            particle_positions_and_collider_bits: &particle_positions_and_collider_bits,
            particle_velocities: &towards_the_center,
            vertex_positions: &vertex_positions,
            vertex_velocities: &vec![Vector4::zeros(); vertex_positions.len()],
            triangle_indices: &triangle_indices,
            triangle_collider: &triangle_collider,
            triangle_frictions: &triangle_frictions,
            triangle_dampings: &triangle_dampings,

            // hacky: will be computed in check
            vertex_normals: &[],
            triangle_normals: &[],
            triangle_opposites: &[],
        },
        true,
    );
}

#[test]
fn moving_torus_with_friction_and_damping() {
    check_moving_torus(0.4, 0.3);
}

#[test]
fn moving_torus_with_invalid_friction_and_damping() {
    check_moving_torus(-0.4, -0.3);
    check_moving_torus(0.4, 1.5);
}

fn check_moving_torus(friction: f32, damping: f32) {
    let vertex_positions = torus::vertices();
    let triangle_indices = torus::triangles();

    let angular_velocity = Vector3::new(0.3, -1.2, 0.7);
    let translation_velocity = Vector3::new(0.5, 0.2, -0.4);
    let vertex_velocities: Vec<Vector4<f32>> = vertex_positions
        .iter()
        .map(|position| (translation_velocity + angular_velocity.cross(&position.xyz())).push(0.))
        .collect();

    let aabb = Aabb::new(vertex_positions.iter().map(Vector4::xyz));
    let known_to_be_outside = 0x0001_0001;
    let off_the_symmetry_planes = Vector3::new(0.0123, 0.0371, 0.0219);
    let particle_positions_and_collider_bits: Vec<_> = aabb
        .lattice(0.2)
        .1
        .map(|position| PositionAndColliderBits {
            position: position + off_the_symmetry_planes,
            collider_bits: known_to_be_outside,
        })
        .collect();
    let particle_count = particle_positions_and_collider_bits.len();
    let particle_flags = vec![Default::default(); particle_count];
    let particle_velocities: Vec<Vector4<f32>> = particle_positions_and_collider_bits
        .iter()
        .map(|particle| Vector4::new(particle.position.y, -particle.position.z, 0.5, 0.))
        .collect();
    let triangle_collider: Vec<u32> = vec![0; triangle_indices.len()];
    let triangle_frictions = vec![friction; triangle_indices.len()];
    let triangle_dampings = vec![damping; triangle_indices.len()];

    check(
        0.7,
        0.5,
        InputData {
            time_step: 0.01,
            leaf_size: 0.5,
            leaf_threshold: 0,
            particle_flags: &particle_flags,
            particle_positions_and_collider_bits: &particle_positions_and_collider_bits,
            particle_velocities: &particle_velocities,
            vertex_positions: &vertex_positions,
            vertex_velocities: &vertex_velocities,
            triangle_indices: &triangle_indices,
            triangle_collider: &triangle_collider,
            triangle_frictions: &triangle_frictions,
            triangle_dampings: &triangle_dampings,
            vertex_normals: &[],
            triangle_normals: &[],
            triangle_opposites: &[],
        },
        false,
    );
}

fn run(
    settings: Settings,
    input_data: InputData,
) -> (Vec<PositionAndColliderBits>, Vec<Vector4<f32>>) {
    let mut context = get_shared_context();

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
