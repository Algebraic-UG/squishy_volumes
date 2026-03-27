// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    mem::swap,
    num::{NonZeroU32, NonZeroU64},
};

use nalgebra::Vector4;

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
        swap(&mut self.front, &mut self.back);
        self.swapped = !self.swapped;
    }

    pub fn swapped(&self) -> bool {
        self.swapped
    }

    pub fn front(&self) -> wgpu::BufferBinding<'a> {
        self.front.clone()
    }

    pub fn back(&self) -> wgpu::BufferBinding<'a> {
        self.back.clone()
    }
}

pub fn find_x_y_z(workgroup_count: u32) -> [u32; 3] {
    let root = (workgroup_count as f64).powf(1. / 3.).floor() as u32;
    let mut xyz = [root; 3];

    let mut inc_dim = 0;
    while xyz.iter().product::<u32>() < workgroup_count {
        xyz[inc_dim] += 1;
        inc_dim += 1;
        inc_dim %= 3;
    }

    xyz
}
