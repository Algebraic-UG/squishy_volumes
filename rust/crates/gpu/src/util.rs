// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

use murmur3::murmur3_32;
use rand::{SeedableRng as _, rngs::ChaCha8Rng, seq::SliceRandom as _};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;

use nalgebra::{Matrix3, Vector3, Vector4};

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
        while hash_table[slot as usize] != 0
            && node_ids_and_collider_bits[(hash_table[slot as usize] - 1) as usize].node_id
                != *node_id
        {
            slot += 1;
            slot &= table_mask;
        }
        hash_table[slot as usize] = node_index as u32 + 1;
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

pub fn create_bind_group<'a>(
    device: &wgpu::Device,
    compiled_module: &CompiledModule,
    entries: impl IntoIterator<Item = wgpu::BufferBinding<'a>>,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: compiled_module.label,
        layout: &compiled_module.bind_group_layout,
        entries: &entries
            .into_iter()
            .enumerate()
            .map(|(binding, entry)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: wgpu::BindingResource::Buffer(entry),
            })
            .collect::<Vec<_>>(),
    })
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
    position.map(|c| (c / grid_node_size).round() as i32 - 1)
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
    cell_size: f32,
    time_step: f32,
    scatter::InputData {
        contributor_offsets,
        contributors,
        node_ids_and_collider_bits,
        particle_tmp,
    }: scatter::InputData,
) -> HashMap<Vector3<i32>, Vector4<f32>> {
    todo!()
}

pub fn collect_on_cpu(
    cell_size: f32,
    time_step: f32,
    scatter::InputData {
        contributor_offsets,
        contributors,
        node_ids_and_collider_bits,
        particle_tmp,
    }: scatter::InputData,
    grid: HashMap<Vector3<i32>, Vector4<f32>>,
) -> (
    Vec<Vector3<f32>>,
    Vec<Matrix3<f32>>,
    Vec<Vector3<f32>>,
    Vec<Matrix3<f32>>,
) {
    todo!()
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
        let hash = node_id_to_murmur(&node.node_id);
        let mut slot = hash & mask;
        while hash_table[slot as usize] != 0 {
            slot += 1;
            slot &= mask;
        }
        hash_table[slot as usize] = index as u32 + 1;
    }
    hash_table
}
