// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

use itertools::izip;
use murmur3::murmur3_32;
use rand::{SeedableRng as _, rngs::ChaCha8Rng, seq::SliceRandom as _};
use rustc_hash::FxHashMap;
use squishy_volumes_util::collider_bits;
use squishy_volumes_util::mesh::{DistanceResult, distance_to_triangle, segment_distance_result};
use squishy_volumes_util::triangle::Triangle;
use squishy_volumes_util::{first_piola_stress_inviscid, first_piola_stress_neo_hookean};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::iter::once;

use nalgebra::{Matrix3, Matrix4, Vector3, Vector4};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq)]
pub struct Block {
    pub nodes: [Vector4<f32>; 8],
}

pub fn bind_group_layout_entry<T: AllowedInBinding>(
    binding: u32,
    read_only: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            min_binding_size: Some(T::MIN_BINDING_SIZE),
            has_dynamic_offset: false,
        },
        count: None,
    }
}

pub fn shuffle<T>(v: &mut [T], seed: u64) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    v.shuffle(&mut rng);
}

pub fn prefix_sum_on_cpu(input: &[u32]) -> Vec<u32> {
    input
        .iter()
        .scan(0, |prefix_sum, item| {
            let result = Some(*prefix_sum);
            *prefix_sum += item;
            result
        })
        .collect()
}

pub fn sort_on_cpu(indices: &[u32], keys: &[u32]) -> Vec<u32> {
    let mut indices = indices.to_vec();
    indices.sort_by_key(|index| keys[*index as usize]);
    indices
}

pub fn i32_to_u32_offset(x: i32) -> u32 {
    (x as u32) ^ 0x8000_0000
}

pub fn u32_to_i32_offset(x: u32) -> i32 {
    (x as i32) ^ 0x8000_0000u32 as i32
}

pub fn node_id_to_murmur(node_id: &Vector3<i32>) -> u32 {
    let node_id = node_id.map(i32_to_u32_offset);
    let mut bytes = [0u8; 12];
    bytes[0..4].copy_from_slice(&node_id.x.to_le_bytes());
    bytes[4..8].copy_from_slice(&node_id.y.to_le_bytes());
    bytes[8..12].copy_from_slice(&node_id.z.to_le_bytes());
    murmur3_32(&mut Cursor::new(bytes), 0).unwrap()
}

pub fn node_id_and_collider_bits_to_murmur(
    NodeIdAndColliderBits {
        node_id,
        collider_bits,
    }: &NodeIdAndColliderBits,
) -> u32 {
    let node_id = node_id.map(i32_to_u32_offset);
    let mut bytes = [0u8; 12];
    bytes[0..4].copy_from_slice(&node_id.x.to_le_bytes());
    bytes[4..8].copy_from_slice(&node_id.y.to_le_bytes());
    bytes[8..12].copy_from_slice(&node_id.z.to_le_bytes());
    murmur3_32(&mut Cursor::new(bytes), *collider_bits).unwrap()
}

pub fn build_hash_table_on_cpu(node_ids_and_collider_bits: &[NodeIdAndColliderBits]) -> Vec<u32> {
    let mut hash_table: Vec<u32> =
        vec![0; (node_ids_and_collider_bits.len() * 2).next_power_of_two()];

    let table_mask = hash_table.len() as u32 - 1;
    for (node_index, node_id_and_collider_bits) in node_ids_and_collider_bits.iter().enumerate() {
        let hash = node_id_and_collider_bits_to_murmur(node_id_and_collider_bits);
        let mut slot = hash & table_mask;
        while hash_table[slot as usize] != 0 {
            slot += 1;
            slot &= table_mask;
        }
        hash_table[slot as usize] = node_index as u32 + 1;
    }
    hash_table
}

pub fn build_hash_table_multi_on_cpu(
    node_ids_and_collider_bits: &[NodeIdAndColliderBits],
) -> Vec<u32> {
    let mut hash_table: Vec<u32> =
        vec![0; (node_ids_and_collider_bits.len() * 2).next_power_of_two()];

    let table_mask = hash_table.len() as u32 - 1;
    for (node_index, NodeIdAndColliderBits { node_id, .. }) in
        node_ids_and_collider_bits.iter().enumerate()
    {
        let hash = node_id_to_murmur(node_id);
        let mut slot = hash & table_mask;
        loop {
            if hash_table[slot as usize] == 0 {
                hash_table[slot as usize] = node_index as u32 + 1;
                break;
            }

            if node_ids_and_collider_bits[(hash_table[slot as usize] - 1) as usize].node_id
                == *node_id
            {
                break;
            }

            slot += 1;
            slot &= table_mask;
        }
    }
    hash_table
}

#[macro_export]
macro_rules! let_buffer {
    ($device:expr, $name:ident<$ty:ty> ($count:expr, $usage:expr)) => {
        let $name = $device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(stringify!($name)),
            size: $count as u64 * <$ty>::MIN_BINDING_SIZE.get(),
            usage: $usage,
            mapped_at_creation: false,
        });
    };
}

pub fn block_offset(block: u32) -> Vector4<i32> {
    Vector4::new(
        if (block & 1) == 1 { 1 } else { 0 },
        if (block & 2) == 2 { 1 } else { 0 },
        if (block & 4) == 4 { 1 } else { 0 },
        0,
    )
}

// TODO: should this return Vector3?
pub fn gpu_grid_to_cpu_grid(block_ids: &[Vector4<i32>]) -> Vec<Vector4<i32>> {
    block_ids
        .iter()
        .flat_map(|block_id| {
            (0..8).map(|node| {
                block_id.xyz().push(0) * 2 - Vector4::new(1, 1, 1, 0) + block_offset(node)
            })
        })
        .collect()
}

pub struct CountsCountArgs {
    pub workgroup_size: u32,
    pub subgroup_size: u32,
    pub dispatch_limit: u32,
    pub counter: u32,
    pub len: u32,
}

pub fn counts_count(
    CountsCountArgs {
        workgroup_size,
        subgroup_size,
        dispatch_limit,
        counter,
        len,
    }: CountsCountArgs,
) -> u32 {
    let subgroups_per_workgroup = workgroup_size / subgroup_size;
    let actual_workgroup_count = Indirect::new(DispatchSettings {
        workgroup_size: workgroup_size.try_into().unwrap(),
        dispatch_limit: dispatch_limit.try_into().unwrap(),
        len,
    })
    .workgroup_count();
    actual_workgroup_count * subgroups_per_workgroup * counter
}

pub fn position_to_low_node(grid_node_size: f32, position: &Vector3<f32>) -> Vector3<i32> {
    position.map(|c| (c / grid_node_size).round_ties_even() as i32 - 1)
}

pub fn prepare_tmp_on_cpu(
    grid_node_size: f32,
    time_step: f32,
    prepare_tmp::InputData {
        particle_masses,
        particle_initial_volumes,
        particle_flags,
        particle_parameters,
        particle_positions_and_collider_bits,
        particle_position_gradients,
        particle_velocities,
        particle_velocity_gradients,
    }: prepare_tmp::InputData,
) -> Vec<Matrix4<f32>> {
    use particle_parameters::{Fluid, Host, Solid};
    let scaling = time_step * 4. / (grid_node_size * grid_node_size);

    izip!(
        particle_masses,
        particle_initial_volumes,
        particle_flags,
        particle_parameters,
        particle_positions_and_collider_bits,
        particle_position_gradients,
        particle_velocities,
        particle_velocity_gradients,
    )
    .map(
        |(
            mass,
            initial_volume,
            flags,
            paramters,
            position_and_collider_bits,
            position_gradient,
            velocity,
            velocity_gradient,
        )|
         -> Matrix4<f32> {
            let position_gradient: Matrix3<f32> = position_gradient.fixed_view::<3, 3>(0, 0).into();
            let stress = match Host::from((*flags, *paramters)) {
                Host::Solid(Solid {
                    mu,
                    lambda,
                    viscosity: _,  // TODO
                    sand_alpha: _, // TODO
                }) => first_piola_stress_neo_hookean(mu, lambda, &position_gradient),
                Host::Fluid(Fluid {
                    exponent,
                    bulk_modulus,
                    viscosity: _, // TODO
                }) => first_piola_stress_inviscid(bulk_modulus, exponent, &position_gradient),
            };

            let matrix_part = velocity_gradient.fixed_view::<3, 3>(0, 0) * *mass
                - stress * position_gradient.transpose() * scaling * *initial_volume;
            let vector_part = velocity.xyz() * *mass;
            let position_part = position_and_collider_bits.position / grid_node_size;

            Matrix4::from_columns(&[
                matrix_part.column(0).push(position_part.x),
                matrix_part.column(1).push(position_part.y),
                matrix_part.column(2).push(position_part.z),
                vector_part.push(*mass),
            ])
        },
    )
    .collect()
}

pub fn kernel_linear(x: f32) -> f32 {
    let x = x.abs();
    if x < 1. { 1. - x } else { 0. }
}

pub fn kernel_quadratic(x: f32) -> f32 {
    let x = x.abs();
    if x < 1. / 2. {
        3. / 4. - x * x
    } else if x < 3. / 2. {
        1. / 2. * (3. / 2. - x) * (3. / 2. - x)
    } else {
        0.
    }
}

pub fn kernel_cubic(x: f32) -> f32 {
    let x = x.abs();
    if x < 1. {
        1. / 2. * x * x * x - x * x + 2. / 3.
    } else if x < 2. {
        1. / 6. * (2. - x) * (2. - x) * (2. - x)
    } else {
        0.
    }
}

pub fn scatter_on_cpu(
    grid_node_size: f32,
    scatter::InputData {
        contributor_offsets,
        contributors,
        node_ids_and_collider_bits,
        particle_tmp,
    }: scatter::InputData,
) -> Vec<Vector4<f32>> {
    node_ids_and_collider_bits
        .iter()
        .zip(
            contributor_offsets.iter().zip(
                contributor_offsets
                    .iter()
                    .skip(1)
                    .chain(once(&(contributors.len() as u32))),
            ),
        )
        .map(
            |(NodeIdAndColliderBits { node_id, .. }, contributor_range)| {
                let normalized_node_position = node_id.map(|c| c as f32);
                println!("{contributor_range:?}");
                (*contributor_range.0 as usize..*contributor_range.1 as usize)
                    .map(|contributor_index| {
                        let tmp: &Matrix4<f32> =
                            &particle_tmp[contributors[contributor_index] as usize];
                        let matrix_part = tmp.fixed_view::<3, 3>(0, 0);
                        let vector_part = tmp.fixed_view::<3, 1>(0, 3);
                        let normalized_particle_position = tmp.fixed_view::<1, 3>(3, 0).transpose();
                        let mass = tmp[(3, 3)];

                        let to_node_normalized =
                            normalized_node_position - normalized_particle_position;
                        let weight: f32 = kernel_quadratic(to_node_normalized.x)
                            * kernel_quadratic(to_node_normalized.y)
                            * kernel_quadratic(to_node_normalized.z);

                        (vector_part + matrix_part * to_node_normalized * grid_node_size).push(mass)
                            * weight
                    })
                    .sum()
            },
        )
        .collect()
}

pub fn collect_on_cpu(
    grid_node_size: f32,
    time_step: f32,
    collect::InputData {
        node_ids_and_collider_bits,
        node_momentums,
        particle_positions_and_collider_bits,
        particle_position_gradients,
        particle_velocities,
        particle_velocity_gradients,
    }: collect::InputData,
) -> collect::OutputData {
    let map: FxHashMap<NodeIdAndColliderBits, Vector4<f32>> = node_ids_and_collider_bits
        .iter()
        .cloned()
        .zip(node_momentums.iter().cloned())
        .collect();

    let mut particle_positions_and_collider_bits = particle_positions_and_collider_bits.to_vec();
    let mut particle_position_gradients = particle_position_gradients.to_vec();
    let mut particle_velocities = particle_velocities.to_vec();
    let mut particle_velocity_gradients = particle_velocity_gradients.to_vec();
    izip!(
        &mut particle_positions_and_collider_bits,
        &mut particle_position_gradients,
        &mut particle_velocities,
        &mut particle_velocity_gradients,
    )
    .for_each(
        |(
            PositionAndColliderBits {
                position,
                collider_bits,
            },
            position_gradient,
            velocity,
            velocity_gradient,
        )| {
            let mut velocity = velocity.fixed_view_mut::<3, 1>(0, 0);
            let mut velocity_gradient = velocity_gradient.fixed_view_mut::<3, 3>(0, 0);

            let normalized_position = *position / grid_node_size;
            let low_node = normalized_position.map(|c| c.round_ties_even() as i32 - 1);

            velocity.fill(0.);
            velocity_gradient.fill(0.);
            for x in 0..3 {
                for y in 0..3 {
                    for z in 0..3 {
                        let node_id = low_node + Vector3::new(x, y, z);
                        let node_momentum = map[&NodeIdAndColliderBits {
                            node_id,
                            collider_bits: *collider_bits,
                        }];

                        if node_momentum.w == 0. {
                            continue;
                        }
                        let to_node_normalized = node_id.map(|c| c as f32) - normalized_position;
                        let weight: f32 = kernel_quadratic(to_node_normalized.x)
                            * kernel_quadratic(to_node_normalized.y)
                            * kernel_quadratic(to_node_normalized.z);
                        let tmp = node_momentum.xyz() * (weight / node_momentum.w);

                        velocity += tmp;
                        velocity_gradient += Matrix3::from_columns(&[
                            tmp * to_node_normalized.x,
                            tmp * to_node_normalized.y,
                            tmp * to_node_normalized.z,
                        ]) * grid_node_size;
                    }
                }
            }
            velocity_gradient *= 4. / (grid_node_size * grid_node_size);

            *position += velocity * time_step;
            let mut position_gradient = position_gradient.fixed_view_mut::<3, 3>(0, 0);
            position_gradient +=
                velocity_gradient * position_gradient.fixed_view::<3, 3>(0, 0) * time_step;
        },
    );

    collect::OutputData {
        particle_positions_and_collider_bits,
        particle_position_gradients,
        particle_velocities,
        particle_velocity_gradients,
    }
}

pub trait Permutation {
    fn permute<T: Clone>(&self, to_permute: &[T]) -> Vec<T>;
}

impl Permutation for &[u32] {
    fn permute<T: Clone>(&self, to_permute: &[T]) -> Vec<T> {
        assert_eq!(self.len(), to_permute.len());
        self.iter()
            .map(|&index| to_permute[index as usize].clone())
            .collect()
    }
}

pub fn get_contributors(
    grid_node_size: f32,
    positions_and_collider_bits: &[PositionAndColliderBits],
) -> HashMap<NodeIdAndColliderBits, Vec<u32>> {
    let mut map: HashMap<NodeIdAndColliderBits, Vec<u32>> = Default::default();
    for (
        particle_index,
        PositionAndColliderBits {
            position,
            collider_bits,
        },
    ) in positions_and_collider_bits.iter().enumerate()
    {
        let low_node = position_to_low_node(grid_node_size, position);
        for x in 0..3 {
            for y in 0..3 {
                for z in 0..3 {
                    map.entry(NodeIdAndColliderBits {
                        node_id: low_node + Vector3::new(x, y, z),
                        collider_bits: *collider_bits,
                    })
                    .or_default()
                    .push(particle_index as u32);
                }
            }
        }
    }
    map
}

pub fn get_node_set(
    grid_node_size: f32,
    positions_and_collider_bits: &[PositionAndColliderBits],
) -> HashSet<NodeIdAndColliderBits> {
    let mut nodes: HashSet<NodeIdAndColliderBits> = Default::default();
    for PositionAndColliderBits {
        position,
        collider_bits,
    } in positions_and_collider_bits
    {
        let low_node = position_to_low_node(grid_node_size, position);
        for x in 0..3 {
            for y in 0..3 {
                for z in 0..3 {
                    nodes.insert(NodeIdAndColliderBits {
                        node_id: low_node + Vector3::new(x, y, z),
                        collider_bits: *collider_bits,
                    });
                }
            }
        }
    }
    nodes
}

pub fn hash_table_on_cpu(
    node_ids_and_collider_bits: &[NodeIdAndColliderBits],
    positions_and_collider_bits: &[PositionAndColliderBits],
) -> Vec<u32> {
    let mut hash_table = vec![0; (positions_and_collider_bits.len() * 27).next_power_of_two()];
    let mask = hash_table.len() as u32 - 1;
    for (index, node) in node_ids_and_collider_bits.iter().enumerate() {
        let hash = node_id_and_collider_bits_to_murmur(node);
        let mut slot = hash & mask;
        while hash_table[slot as usize] != 0 {
            slot += 1;
            slot &= mask;
        }
        hash_table[slot as usize] = index as u32 + 1;
    }
    hash_table
}

pub fn contributors_on_cpu(
    grid_node_size: f32,
    positions_and_collider_bits: &[PositionAndColliderBits],
) -> (Vec<NodeIdAndColliderBits>, Vec<u32>, Vec<u32>) {
    let mut node_ids_and_collider_bits = Vec::new();
    let mut contributor_offsets = Vec::new();
    let mut contributors = Vec::new();
    let mut offset = 0;
    for (node, mut node_contributors) in
        get_contributors(grid_node_size, positions_and_collider_bits).into_iter()
    {
        contributor_offsets.push(offset);
        node_ids_and_collider_bits.push(node);
        offset += node_contributors.len() as u32;
        contributors.append(&mut node_contributors);
    }
    (
        node_ids_and_collider_bits,
        contributor_offsets,
        contributors,
    )
}

pub fn collide_on_cpu(
    time_step: f32,
    accept_distance: f32,
    forget_distance: f32,
    collide::InputData {
        particle_positions_and_collider_bits,
        particle_velocities,
        vertex_positions,
        triangle_indices,
        triangle_collider,
        triangle_frictions: _,
        vertex_normals,
        triangle_normals,
        triangle_opposites, // TODO
        ..
    }: collide::InputData,
) -> (Vec<PositionAndColliderBits>, Vec<Vector3<f32>>) {
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
            .zip(triangle_normals)
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
    (
        cpu_particle_positions_and_collider_bits,
        cpu_particle_velocites,
    )
}
