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

pub struct OffsetsToIndirect {
    compiled_module: CompiledModule,
}

pub struct OffsetsToIndirectSettings {
    pub workgroup_size: u32,
    pub dispatch_limit: u32,
}

pub struct OffsetsToIndirectBufferInput<'a> {
    pub prefix_sums: &'a [u32],
}

pub struct OffsetsToIndirectBuffers {
    pub prefix_sums: wgpu::Buffer,
    pub limits: wgpu::Buffer,
    pub indirect: wgpu::Buffer,
}
pub struct OffsetsToIndirectBufferBindings<'a> {
    pub prefix_sums: wgpu::BufferBinding<'a>,
    pub limits: wgpu::BufferBinding<'a>,
    pub indirect: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a OffsetsToIndirectBuffers> for OffsetsToIndirectBufferBindings<'a> {
    fn from(
        OffsetsToIndirectBuffers {
            prefix_sums,
            limits,
            indirect,
        }: &'a OffsetsToIndirectBuffers,
    ) -> Self {
        Self {
            prefix_sums: prefix_sums.as_entire_buffer_binding(),
            limits: limits.as_entire_buffer_binding(),
            indirect: indirect.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for OffsetsToIndirect {
    type Settings = OffsetsToIndirectSettings;
    type Parameters = ();
    type BufferInput<'a> = OffsetsToIndirectBufferInput<'a>;
    type Buffers = OffsetsToIndirectBuffers;
    type BufferBindings<'a> = OffsetsToIndirectBufferBindings<'a>;

    fn new(
        context: &GpuContext,
        Self::Settings {
            workgroup_size,
            dispatch_limit,
        }: Self::Settings,
    ) -> Self {
        let device = context.device();

        let label = Some("sum_to_indirect");

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
            module: &device.create_shader_module(wgpu::include_wgsl!("offsets_to_indirect.wgsl")),
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
        Self::BufferInput { prefix_sums }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let device = context.device();
        let prefix_sums = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("prefix_sums"),
            contents: bytemuck::cast_slice(prefix_sums),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let limits = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("limits"),
            size: u32::MIN_BINDING_SIZE.get(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let indirect = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("indirect"),
            size: 3 * u32::MIN_BINDING_SIZE.get(),
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
            prefix_sums,
            limits,
            indirect,
        }: Self::BufferBindings<'a>,
        _: Self::Parameters,
    ) {
        let device = context.device();

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
