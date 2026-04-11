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

pub struct GenerateIndices {
    compiled_module: CompiledModule,
}

pub struct GenerateIndicesSettings {
    pub workgroup_size: u32,
}

pub struct GenerateIndicesBufferInput<'a> {
    pub count: u32,
    pub limits: &'a [u32],
    pub indirect: &'a [u32],
}

pub struct GenerateIndicesBuffers {
    pub indices: wgpu::Buffer,
    pub limits: wgpu::Buffer,
    pub indirect: wgpu::Buffer,
}

pub struct GenerateIndicesBufferBindings<'a> {
    pub indices: wgpu::BufferBinding<'a>,
    pub limits: wgpu::BufferBinding<'a>,
    pub indirect: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a GenerateIndicesBuffers> for GenerateIndicesBufferBindings<'a> {
    fn from(
        GenerateIndicesBuffers {
            indices,
            limits,
            indirect,
        }: &'a GenerateIndicesBuffers,
    ) -> Self {
        Self {
            indices: indices.as_entire_buffer_binding(),
            limits: limits.as_entire_buffer_binding(),
            indirect: indirect.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for GenerateIndices {
    type Settings = GenerateIndicesSettings;
    type Parameters = ();
    type BufferInput<'a> = GenerateIndicesBufferInput<'a>;
    type Buffers = GenerateIndicesBuffers;
    type BufferBindings<'a> = GenerateIndicesBufferBindings<'a>;

    fn new(context: &GpuContext, Self::Settings { workgroup_size }: Self::Settings) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size > 0);
        assert!(workgroup_size.is_multiple_of(subgroup_size));

        let device = context.device();

        let label = Some("generate_indices");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[
                bind_group_layout_entry::<u32>(0, true),
                bind_group_layout_entry::<u32>(1, false),
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
            module: &device.create_shader_module(wgpu::include_wgsl!("generate_indices.wgsl")),
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
            count,
            limits,
            indirect,
        }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let device = context.device();

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

        let_buffer!(device, indices<u32>(count, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));

        Self::Buffers {
            indices,
            limits,
            indirect,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            indices,
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
                    resource: wgpu::BindingResource::Buffer(limits),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(indices),
                },
            ],
        });

        compute_pass.set_pipeline(&self.compiled_module.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        compute_pass.dispatch_workgroups_indirect(indirect.buffer, 0);
    }
}
