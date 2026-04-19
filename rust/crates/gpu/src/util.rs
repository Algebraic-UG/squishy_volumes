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
use std::collections::HashSet;
use std::io::Cursor;
use std::iter::once;
use std::marker::PhantomData;
use std::num::{NonZeroU32, NonZeroU64};

use nalgebra::Vector4;

pub const MAX_NUM_PARTICLES: u32 = 10000000;

pub struct CompiledModule {
    pub label: Option<&'static str>,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub compute_pipeline: wgpu::ComputePipeline,
}

pub struct CompiledModuleSettings<'a, BindGroupEntries, Constants> {
    pub device: &'a wgpu::Device,
    pub bind_group_entries: BindGroupEntries,
    pub immediate_size: u32,
    pub constants: Constants,
}

impl CompiledModule {
    pub fn new<BindGroupEntries, Constants>(
        label: &'static str,
        shader_module_descriptor: wgpu::ShaderModuleDescriptor,
        CompiledModuleSettings {
            device,
            bind_group_entries,
            immediate_size,
            constants,
        }: CompiledModuleSettings<BindGroupEntries, Constants>,
    ) -> Self
    where
        BindGroupEntries: IntoIterator<Item = (NonZeroU64, bool)>,
        Constants: IntoIterator<Item = (&'static str, f64)>,
    {
        let label = Some(label);
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &bind_group_entries
                .into_iter()
                .enumerate()
                .map(
                    |(binding, (min_binding_size, read_only))| wgpu::BindGroupLayoutEntry {
                        binding: binding as u32,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only },
                            min_binding_size: Some(min_binding_size),
                            has_dynamic_offset: false,
                        },
                        count: None,
                    },
                )
                .collect::<Vec<_>>(),
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label,
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size,
                }),
            ),
            module: &device.create_shader_module(shader_module_descriptor),
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &constants.into_iter().collect::<Vec<_>>(),
                ..Default::default()
            },
            cache: None,
        });

        CompiledModule {
            label,
            bind_group_layout,
            compute_pipeline,
        }
    }
}

#[macro_export]
macro_rules! let_compiled_module {
    ($name:ident, $settings:expr) => {
        let $name = CompiledModule::new(
            stringify!($name),
            wgpu::include_wgsl!(concat!(stringify!($name), ".wgsl")),
            $settings,
        );
    };
}

pub fn binding_size(binding: &wgpu::BufferBinding) -> NonZeroU64 {
    binding.size.unwrap_or_else(|| {
        assert!(binding.buffer.size() > binding.offset);
        NonZeroU64::try_from(binding.buffer.size() - binding.offset).unwrap()
    })
}

pub trait AllowedInBinding: Sized {
    const MIN_BINDING_SIZE: NonZeroU64 = NonZeroU64::new(size_of::<Self>() as u64).unwrap();
    const ALIGNMENT: NonZeroU64 = NonZeroU64::new(size_of::<Self>() as u64).unwrap();
}

impl AllowedInBinding for u32 {}
impl AllowedInBinding for f32 {}
impl AllowedInBinding for Vector4<f32> {}
impl AllowedInBinding for Vector4<i32> {}
impl AllowedInBinding for Vector4<u32> {}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq)]
pub struct Indirect {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub len: u32,
}

pub struct IndirectSettings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub len: u32,
}
impl Indirect {
    pub fn new(
        IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len,
        }: IndirectSettings,
    ) -> Self {
        let workgroup_size = workgroup_size.get();
        let dispatch_limit = dispatch_limit.get();

        let workgroup_count = len.div_ceil(workgroup_size);

        let x = workgroup_count.min(dispatch_limit);
        let y = workgroup_count.div_ceil(dispatch_limit).min(dispatch_limit);
        let z = workgroup_count
            .div_ceil(dispatch_limit * dispatch_limit)
            .min(dispatch_limit);

        Self { x, y, z, len }
    }

    pub fn workgroup_count(&self) -> u32 {
        self.x * self.y * self.z
    }
}

impl AllowedInBinding for Indirect {}

pub struct DynArray<T> {
    bytes: Vec<u8>,
    _marker: PhantomData<T>,
}

impl<T: AllowedInBinding + bytemuck::Pod> DynArray<T> {
    pub fn new(len: &u32, values: &[u32]) -> Self {
        let mut bytes = bytemuck::bytes_of(len).to_vec();
        // padding
        bytes.resize(Self::values_start(), 0);
        bytes.extend_from_slice(bytemuck::cast_slice(values));
        Self {
            bytes,
            _marker: PhantomData,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
            _marker: PhantomData,
        }
    }

    pub fn len(&self) -> &u32 {
        bytemuck::from_bytes(&self.bytes.as_slice()[0..4])
    }

    pub fn as_slice(&self) -> &[T] {
        bytemuck::cast_slice(&self.bytes.as_slice()[Self::values_start()..])
    }

    fn values_start() -> usize {
        u32::MIN_BINDING_SIZE
            .get()
            .next_multiple_of(T::ALIGNMENT.get()) as usize
    }
}

impl<T: AllowedInBinding + bytemuck::Pod> AllowedInBinding for DynArray<T> {
    const MIN_BINDING_SIZE: NonZeroU64 = NonZeroU64::new(2 * T::MIN_BINDING_SIZE.get()).unwrap();
    const ALIGNMENT: NonZeroU64 = T::ALIGNMENT;
}

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

pub fn color_cells_on_cpu(
    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
    subgroup_size: u32,
    cells: &[Vector4<i32>],
) -> (Vec<Indirect>, Vec<Indirect>, Vec<u32>) {
    let keys: Vec<u32> = cells_to_colorkeys_on_cpu(cells);
    println!("keys: {keys:?}");

    let counts = (0..8)
        .map(|color| keys.iter().filter(|key| **key == color).count() as u32)
        .collect::<Vec<_>>();
    println!("counts: {counts:?}");
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
                workgroup_size: workgroup_size.try_into().unwrap(),
                dispatch_limit: dispatch_limit.try_into().unwrap(),
                len: *count,
            });
            let mut indirect_color_batch = Indirect::new(IndirectSettings {
                workgroup_size: workgroup_size.try_into().unwrap(),
                dispatch_limit: dispatch_limit.try_into().unwrap(),
                len: *count * subgroup_size,
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
    limits: &[u32],
    cell_ids: &[Vector4<i32>],
    cell_owns: &[u32],
) -> Vec<Vector4<i32>> {
    cell_ids
        .iter()
        .zip(cell_owns)
        .take(*limits.last().unwrap() as usize)
        .flat_map(move |(cell_id, cell_own)| {
            (0..8)
                .filter(move |block| cell_own & (1 << block) > 0)
                .flat_map(move |block| {
                    let node_id = (cell_id + block_offset(block)) * 2 - Vector4::new(1, 1, 1, 0);
                    (0..2).flat_map(move |x| {
                        (0..2).flat_map(move |y| {
                            (0..2).map(move |z| node_id + Vector4::new(x, y, z, 0))
                        })
                    })
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
        let cell_id = position.map(|c| (c / cell_size).floor() as i32);
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
