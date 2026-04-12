// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use wgpu::util::DeviceExt as _;

use super::*;

pub struct RecycleToIndirect {
    compiled_module: CompiledModule,
}

pub struct RecycleToIndirectSettings {
    pub workgroup_size: u32,
    pub dispatch_limit: u32,
}

pub struct RecycleToIndirectBufferInput<'a> {
    pub limits: &'a [u32],
    pub prefix_sums: &'a [u32],
}

pub struct RecycleToIndirectBuffers {
    pub indirect: wgpu::Buffer,
    pub limits: wgpu::Buffer,
    pub prefix_sums: wgpu::Buffer,
}

pub struct RecycleToIndirectBufferBindings<'a> {
    pub indirect: wgpu::BufferBinding<'a>,
    pub limits: wgpu::BufferBinding<'a>,
    pub prefix_sums: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a RecycleToIndirectBuffers> for RecycleToIndirectBufferBindings<'a> {
    fn from(
        RecycleToIndirectBuffers {
            indirect,
            limits,
            prefix_sums,
        }: &'a RecycleToIndirectBuffers,
    ) -> Self {
        Self {
            indirect: indirect.as_entire_buffer_binding(),
            limits: limits.as_entire_buffer_binding(),
            prefix_sums: prefix_sums.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for RecycleToIndirect {
    type Settings = RecycleToIndirectSettings;
    type Parameters = ();
    type BufferInput<'a> = RecycleToIndirectBufferInput<'a>;
    type Buffers = RecycleToIndirectBuffers;
    type BufferBindings<'a> = RecycleToIndirectBufferBindings<'a>;

    fn new(
        context: &GpuContext,
        Self::Settings {
            workgroup_size,
            dispatch_limit,
        }: Self::Settings,
    ) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(subgroup_size >= 2u32.pow(3));

        let device = context.device();

        let label = Some("recycle_to_indirect");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[
                bind_group_layout_entry::<u32>(0, true),
                bind_group_layout_entry::<u32>(1, false),
                bind_group_layout_entry::<u32>(2, false),
            ],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label,
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    ..Default::default()
                }),
            ),
            module: &device.create_shader_module(wgpu::include_wgsl!("recycle_to_indirect.wgsl")),
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[
                    ("WORKGROUP_SIZE", workgroup_size as f64),
                    ("DISPATCH_LIMIT", dispatch_limit as f64),
                ],
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
            limits,
            prefix_sums,
        }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        assert!(prefix_sums.len().is_multiple_of(8));

        let device = context.device();
        let prefix_sums = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("prefix_sums"),
            contents: bytemuck::cast_slice(prefix_sums),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let limits = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("limits"),
            contents: bytemuck::cast_slice(limits),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let indirect = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("indirect"),
            size: 8 * 3 * u32::MIN_BINDING_SIZE.get(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self::Buffers {
            indirect,
            limits,
            prefix_sums,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            indirect,
            limits,
            prefix_sums,
        }: Self::BufferBindings<'a>,
        _: Self::Parameters,
    ) {
        let device = context.device();
        assert!(
            elements_in_binding::<u32>(&prefix_sums)
                .get()
                .is_multiple_of(8)
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.compiled_module.label,
            layout: &self.compiled_module.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(prefix_sums.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(limits.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(indirect.clone()),
                },
            ],
        });

        compute_pass.set_pipeline(&self.compiled_module.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        compute_pass.dispatch_workgroups(1, 1, 1);
    }
}
