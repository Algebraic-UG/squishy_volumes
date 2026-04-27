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
use std::collections::HashSet;
use std::io::Cursor;
use std::iter::once;
use std::num::NonZeroU32;

use nalgebra::Vector4;

pub const MAX_NUM_PARTICLES: u32 = 1000000;

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

pub fn sort_positions_into_cells_on_cpu(
    indices: &[u32],
    positions: &[Vector4<f32>],
    cell_size: f32,
) -> Vec<u32> {
    let mut indices = indices.to_vec();
    indices.sort_by(|a, b| {
        let a = position_to_cell(cell_size, &positions[*a as usize]);
        let b = position_to_cell(cell_size, &positions[*b as usize]);
        a.x.cmp(&b.x).then(a.y.cmp(&b.y)).then(a.z.cmp(&b.z))
    });
    indices
}

pub fn i32_to_u32_offset(x: i32) -> u32 {
    (x as u32) ^ 0x8000_0000
}

pub fn u32_to_i32_offset(x: u32) -> i32 {
    (x as i32) ^ 0x8000_0000u32 as i32
}

pub fn positions_to_keys_on_cpu(
    positions: &[Vector4<f32>],
    cell_size: f32,
    dimension: u32,
) -> Vec<u32> {
    positions
        .iter()
        .map(|position| {
            i32_to_u32_offset(position_to_cell(cell_size, position)[dimension as usize])
        })
        .collect()
}

pub fn cell_to_murmur(cell: &Vector4<i32>) -> u32 {
    let cell = cell.map(i32_to_u32_offset);
    let mut bytes = [0u8; 12];
    bytes[0..4].copy_from_slice(&cell.x.to_le_bytes());
    bytes[4..8].copy_from_slice(&cell.y.to_le_bytes());
    bytes[8..12].copy_from_slice(&cell.z.to_le_bytes());
    murmur3_32(&mut Cursor::new(bytes), 0).unwrap()
}

pub fn cells_to_murmur_on_cpu(cells: &[Vector4<i32>]) -> Vec<u32> {
    cells.iter().map(cell_to_murmur).collect()
}

pub fn find_cell_boundaries_on_cpu(positions: &[Vector4<f32>], cell_size: f32) -> Vec<u32> {
    positions
        .iter()
        .zip(positions.iter().skip(1))
        .map(|(position, next_position)| {
            if position_to_cell(cell_size, position) != position_to_cell(cell_size, next_position) {
                1
            } else {
                0
            }
        })
        .chain(once(1))
        .collect()
}

pub fn build_cells_on_cpu(
    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
    cell_size: f32,
    positions: &[Vector4<f32>],
    prefixed_boundaries: &[u32],
) -> (Vec<Vector4<i32>>, Vec<u32>, Indirect) {
    let mut cell_ids: Vec<Vector4<i32>> = Default::default();
    let mut index_ranges: Vec<u32> = Default::default();
    for (index, position) in positions.iter().enumerate() {
        if index + 1 != positions.len()
            && prefixed_boundaries[index] == prefixed_boundaries[index + 1]
        {
            continue;
        }
        cell_ids.push(position_to_cell(cell_size, position));
        index_ranges.push(index as u32 + 1);
    }
    let indirect = Indirect::new(IndirectSettings {
        workgroup_size,
        dispatch_limit,
        len: cell_ids.len() as u32,
    });

    (cell_ids, index_ranges, indirect)
}

pub fn build_hash_table_on_cpu(cell_ids: &[Vector4<i32>]) -> (Vec<u32>, Vec<u32>) {
    let mut block_table: Vec<u32> = vec![0; (cell_ids.len() * 8 * 2).next_power_of_two()];
    let mut owns: Vec<u32> = vec![0; cell_ids.len()];
    let table_mask = block_table.len() as u32 - 1;
    let index_mask = (1 << 29) - 1;

    for (index, cell_id) in cell_ids.iter().enumerate() {
        for block in 0..8 {
            let block_and_index = (block << 29) | (index as u32 + 1);

            let block_id = cell_id + block_offset(block);
            let hash = cell_to_murmur(&block_id);
            let mut slot = hash & table_mask;
            loop {
                let old_block_and_index = block_table[slot as usize];
                if old_block_and_index == 0 {
                    block_table[slot as usize] = block_and_index;
                    owns[index] |= 1 << block;
                    break;
                }
                let old_block = old_block_and_index >> 29;
                let old_index = (old_block_and_index & index_mask) - 1;
                if cell_ids[old_index as usize] + block_offset(old_block) == block_id {
                    break;
                }

                slot += 1;
                slot &= table_mask;
            }
        }
    }

    (block_table, owns)
}

pub fn cells_to_colorkeys_on_cpu(cells: &[Vector4<i32>]) -> Vec<u32> {
    cells
        .iter()
        .map(|cell| {
            let ucell = cell.map(i32_to_u32_offset);
            let mut key = 0;
            key |= ucell.z & 1;
            key |= (ucell.y & 1) << 1;
            key |= (ucell.x & 1) << 2;
            key
        })
        .collect()
}

pub fn color_cells_on_cpu(
    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
    subgroup_size: NonZeroU32,
    cells: &[Vector4<i32>],
) -> (Vec<Indirect>, Vec<Indirect>, Vec<u32>) {
    let keys: Vec<u32> = cells_to_colorkeys_on_cpu(cells);

    let counts = (0..8)
        .map(|color| keys.iter().filter(|key| **key == color).count() as u32)
        .collect::<Vec<_>>();
    let prefix_sum: Vec<_> = counts
        .iter()
        .scan(0, |prefix_sum, item| {
            *prefix_sum += item;
            Some(*prefix_sum)
        })
        .collect();
    let (indirect_colors, indirect_colors_batch): (Vec<_>, Vec<_>) = counts
        .iter()
        .zip(prefix_sum)
        .map(|(count, end)| {
            let mut indirect_color = Indirect::new(IndirectSettings {
                workgroup_size,
                dispatch_limit,
                len: *count,
            });
            let mut indirect_color_batch = Indirect::new(IndirectSettings {
                workgroup_size,
                dispatch_limit,
                len: *count * subgroup_size.get(),
            });
            indirect_color.len = end;
            indirect_color_batch.len = end;
            (indirect_color, indirect_color_batch)
        })
        .unzip();
    let indices = sort_on_cpu(&(0..cells.len() as u32).collect::<Vec<_>>(), &keys);

    (indirect_colors, indirect_colors_batch, indices)
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

pub fn gpu_grid_to_cpu_grid(
    indirect: Indirect,
    cell_ids: &[Vector4<i32>],
    cell_owns: &[u32],
) -> Vec<Vector4<i32>> {
    cell_ids
        .iter()
        .zip(cell_owns)
        .take(indirect.len as usize)
        .flat_map(move |(cell_id, cell_own)| {
            (0..8)
                .filter(move |block| cell_own & (1 << block) > 0)
                .flat_map(move |block| {
                    let node_id = (cell_id + block_offset(block)) * 2 - Vector4::new(1, 1, 1, 0);
                    (0..8).map(move |node| node_id.xyz().push(0) + block_offset(node))
                })
        })
        .collect()
}

pub fn grid_on_cpu(
    cell_size: f32,
    indices: &[u32],
    positions: &[Vector4<f32>],
) -> Vec<Vector4<i32>> {
    let mut nodes: HashSet<Vector4<i32>> = Default::default();
    for position in indices.iter().map(|index| positions[*index as usize]) {
        let cell_id = position_to_cell(cell_size, &position);
        for block in 0..8 {
            let node_id = (cell_id + block_offset(block)) * 2 - Vector4::new(1, 1, 1, 0);
            nodes.extend((0..2).flat_map(move |x| {
                (0..2).flat_map(move |y| (0..2).map(move |z| node_id + Vector4::new(x, y, z, 0)))
            }));
        }
    }
    nodes.into_iter().collect()
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
    let actual_workgroup_count = Indirect::new(IndirectSettings {
        workgroup_size: workgroup_size.try_into().unwrap(),
        dispatch_limit: dispatch_limit.try_into().unwrap(),
        len,
    })
    .workgroup_count();
    actual_workgroup_count * subgroups_per_workgroup * counter
}

pub fn position_to_cell(cell_size: f32, position: &Vector4<f32>) -> Vector4<i32> {
    position
        .xyz()
        .map(|c| (c / cell_size + 0.25).floor() as i32)
        .push(0)
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
