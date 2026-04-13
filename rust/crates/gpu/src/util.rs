// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use lazy_static::lazy_static;
use murmur3::murmur3_32;
use rand::{SeedableRng as _, rngs::ChaCha8Rng, seq::SliceRandom as _};
use std::io::Cursor;
use std::iter::once;
use std::num::{NonZeroU32, NonZeroU64};
use std::sync::atomic::AtomicU32;

use nalgebra::Vector4;

pub const MAX_NUM_PARTICLES: u32 = 10000000;

pub struct CompiledModule {
    pub label: Option<&'static str>,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub compute_pipeline: wgpu::ComputePipeline,
}

pub fn binding_size(binding: &wgpu::BufferBinding) -> NonZeroU64 {
    binding.size.unwrap_or_else(|| {
        assert!(binding.buffer.size() > binding.offset);
        NonZeroU64::try_from(binding.buffer.size() - binding.offset).unwrap()
    })
}

pub trait AllowedInBinding: Sized {
    const MIN_BINDING_SIZE: NonZeroU64 = NonZeroU64::new(size_of::<Self>() as u64).unwrap();
}

impl AllowedInBinding for u32 {}
impl AllowedInBinding for f32 {}
impl AllowedInBinding for Vector4<f32> {}
impl AllowedInBinding for Vector4<i32> {}
impl AllowedInBinding for AtomicU32 {}

pub fn elements_in_binding<T: AllowedInBinding>(binding: &wgpu::BufferBinding) -> NonZeroU32 {
    NonZeroU32::try_from((binding_size(binding).get() / T::MIN_BINDING_SIZE.get()) as u32).unwrap()
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

pub struct DoubleBuffer<'a> {
    swapped: bool,
    front: wgpu::BufferBinding<'a>,
    back: wgpu::BufferBinding<'a>,
}

impl<'a> DoubleBuffer<'a> {
    pub fn new(front: wgpu::BufferBinding<'a>, back: wgpu::BufferBinding<'a>) -> Self {
        assert_eq!(binding_size(&front), binding_size(&back));

        Self {
            swapped: false,
            front,
            back,
        }
    }

    pub fn swap(&mut self) {
        self.swapped = !self.swapped;
    }

    pub fn swapped(&self) -> bool {
        self.swapped
    }

    pub fn front(&self) -> wgpu::BufferBinding<'a> {
        if self.swapped() {
            self.back.clone()
        } else {
            self.front.clone()
        }
    }

    pub fn back(&self) -> wgpu::BufferBinding<'a> {
        if self.swapped() {
            self.front.clone()
        } else {
            self.back.clone()
        }
    }
}

lazy_static! {
    static ref PRIMES: Vec<u16> = {
        let mut primes: Vec<u16> = Default::default();
        for i in 2..u16::MAX {
            if primes.iter().all(|prime| !i.is_multiple_of(*prime)) {
                primes.push(i);
            }
        }
        primes
    };
}

/*
pub fn find_x_y_z(workgroup_count: u32) -> [u32; 3] {
    for offset in 0..4 {
        let workgroup_count = workgroup_count + offset;
        let mut best_factors = [None, None];
        for prime in PRIMES.iter() {
            if workgroup_count.is_multiple_of(*prime as u32) {
                best_factors[1] = best_factors[0];
                best_factors[0] = Some(*prime as u32);
            }
        }
        if let [Some(a), Some(b)] = best_factors
            && workgroup_count / a / b < u16::MAX as u32
        {
            assert_eq!(a * b * (workgroup_count / a / b), workgroup_count);
            return [a, b, workgroup_count / a / b];
        }
    }

    let root = (workgroup_count as f64).powf(1. / 3.).floor() as u32;
    let mut xyz = [root; 3];

    let mut inc_dim = 0;
    while xyz.iter().product::<u32>() < workgroup_count {
        xyz[inc_dim] += 1;
        inc_dim += 1;
        inc_dim %= 3;
    }

    xyz
}*/

pub fn find_x_y_z_simple(limit: u32, workgroup_count: u32) -> [u32; 3] {
    [
        workgroup_count.min(limit),
        workgroup_count.div_ceil(limit).min(limit),
        workgroup_count.div_ceil(limit * limit).min(limit),
    ]
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
        let a = positions[*a as usize].map(|c| (c / cell_size).floor() as i32);
        let b = positions[*b as usize].map(|c| (c / cell_size).floor() as i32);
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
        .map(
            |position| i32_to_u32_offset((position[dimension as usize] / cell_size).floor() as i32),
        )
        .collect()
}

pub fn cells_to_murmur_on_cpu(cells: &[Vector4<i32>]) -> Vec<u32> {
    cells
        .iter()
        .map(|cell| {
            let cell = cell.map(i32_to_u32_offset);
            let mut bytes = [0u8; 12];
            bytes[0..4].copy_from_slice(&cell.x.to_le_bytes());
            bytes[4..8].copy_from_slice(&cell.y.to_le_bytes());
            bytes[8..12].copy_from_slice(&cell.z.to_le_bytes());
            murmur3_32(&mut Cursor::new(bytes), 0).unwrap()
        })
        .collect()
}

pub fn find_cell_boundaries_on_cpu(positions: &[Vector4<f32>], cell_size: f32) -> Vec<u32> {
    positions
        .iter()
        .zip(positions.iter().skip(1))
        .map(|(position, next_position)| {
            if position.map(|c| (c / cell_size).floor() as i32)
                != next_position.map(|c| (c / cell_size).floor() as i32)
            {
                1
            } else {
                0
            }
        })
        .chain(once(1))
        .collect()
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
