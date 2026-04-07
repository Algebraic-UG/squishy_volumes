// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use std::sync::atomic::AtomicU32;

use nalgebra::Vector4;
use wgpu::util::DeviceExt as _;

use super::*;

pub struct BuildHashTableColors {
    compiled_module: CompiledModule,
}

pub struct BuildHashTableColorsSettings {
    pub workgroup_size: u32,
}

pub struct BuildHashTableColorsBufferInput<'a> {
    pub cells: &'a [Vector4<i32>],
    pub indices: &'a [u32],
    pub limits: &'a [u32],
    pub indirect: &'a [u32],
}

pub struct BuildHashTableColorsBuffers {
    pub cells: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub limits: wgpu::Buffer,
    pub indirect: wgpu::Buffer,
    pub slots: wgpu::Buffer,
    pub owns: wgpu::Buffer,
}

pub struct BuildHashTableColorsBufferBindings<'a> {
    pub cells: wgpu::BufferBinding<'a>,
    pub indices: wgpu::BufferBinding<'a>,
    pub limits: wgpu::BufferBinding<'a>,
    pub indirect: wgpu::BufferBinding<'a>,
    pub slots: wgpu::BufferBinding<'a>,
    pub owns: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a BuildHashTableColorsBuffers> for BuildHashTableColorsBufferBindings<'a> {
    fn from(
        BuildHashTableColorsBuffers {
            cells,
            indices,
            limits,
            indirect,
            slots,
            owns,
        }: &'a BuildHashTableColorsBuffers,
    ) -> Self {
        Self {
            cells: cells.as_entire_buffer_binding(),
            indices: indices.as_entire_buffer_binding(),
            limits: limits.as_entire_buffer_binding(),
            indirect: indirect.as_entire_buffer_binding(),
            slots: slots.as_entire_buffer_binding(),
            owns: owns.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for BuildHashTableColors {
    type Settings = BuildHashTableColorsSettings;
    type Parameters = ();
    type BufferInput<'a> = BuildHashTableColorsBufferInput<'a>;
    type Buffers = BuildHashTableColorsBuffers;
    type BufferBindings<'a> = BuildHashTableColorsBufferBindings<'a>;

    fn new(context: &GpuContext, Self::Settings { workgroup_size }: Self::Settings) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size > 0);
        assert!(workgroup_size.is_multiple_of(subgroup_size));

        let device = context.device();

        let label = Some("build_hash_table_colors");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[
                bind_group_layout_entry::<Vector4<i32>>(0, true),
                bind_group_layout_entry::<u32>(1, true),
                bind_group_layout_entry::<u32>(2, true),
                bind_group_layout_entry::<AtomicU32>(3, false),
                bind_group_layout_entry::<u32>(4, false),
            ],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label,
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size: 4,
                }),
            ),
            module: &device
                .create_shader_module(wgpu::include_wgsl!("build_hash_table_colors.wgsl")),
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[("WORKGROUP_SIZE", workgroup_size as f64)],
                ..Default::default()
            },
            cache: None,
        });

        let compiled_module = CompiledModule {
            label,
            bind_group_layout,
            compute_pipeline,
        };

        Self { compiled_module }
    }

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput {
            cells,
            indices,
            limits,
            indirect,
        }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let device = context.device();
        let n = cells.len() as u32;
        assert_eq!(cells.len(), indices.len());
        assert_eq!(8, limits.len());
        assert_eq!(8 * 3, indirect.len());

        let cells = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cells"),
            contents: bytemuck::cast_slice(cells),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indices"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let limits = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("limits"),
            contents: bytemuck::cast_slice(limits),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let indirect = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indirect"),
            contents: bytemuck::cast_slice(indirect),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
        });

        let slots = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("slots"),
            size: self.min_table(n) as u64 * AtomicU32::MIN_BINDING_SIZE.get(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let owns = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("owns"),
            size: n as u64 * AtomicU32::MIN_BINDING_SIZE.get(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self::Buffers {
            cells,
            indices,
            limits,
            indirect,
            slots,
            owns,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            cells,
            indices,
            limits,
            indirect,
            slots,
            owns,
        }: &mut Self::BufferBindings<'a>,
        _: &mut Self::Parameters,
    ) {
        let cell_count = elements_in_binding::<Vector4<i32>>(cells);
        assert_eq!(cell_count, elements_in_binding::<u32>(indices));
        assert_eq!(cell_count, elements_in_binding::<u32>(owns));

        let slots_count = elements_in_binding::<AtomicU32>(slots);
        assert!(slots_count >= cell_count); // better if it's much larger ofc

        let device = context.device();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.compiled_module.label,
            layout: &self.compiled_module.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(cells.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(indices.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(limits.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(slots.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(owns.clone()),
                },
            ],
        });

        compute_pass.set_pipeline(&self.compiled_module.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        for color in 0..8u32 {
            compute_pass.set_immediates(0, bytemuck::bytes_of(&color));
            compute_pass.dispatch_workgroups_indirect(
                indirect.buffer,
                indirect.offset + color as u64 * u32::MIN_BINDING_SIZE.get() * 3,
            );
        }
    }
}

impl BuildHashTableColors {
    // control load factor to be at most 0.5
    // TODO: this is way too much for most sparsity patterns
    pub fn min_table(&self, cell_count: u32) -> u32 {
        //(cell_count * 2).next_power_of_two()
        (cell_count * 8 * 2).next_power_of_two()
    }
}
