// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector4;

#[cfg(test)]
mod test;

use wgpu::util::DeviceExt as _;

use super::*;

pub struct PermutePositions {
    workgroup_size: u32,
    compiled_module: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct PermutePositionsSettings {
    pub workgroup_size: u32,
}

pub struct PermutePositionsBufferInput<'a> {
    pub permutation: &'a [u32],
    pub positions: &'a [Vector4<f32>],
}

pub struct PermutePositionsBuffers {
    pub permutation: wgpu::Buffer,
    pub positions_in: wgpu::Buffer,
    pub positions_out: wgpu::Buffer,
}

pub struct PermutePositionsBufferBindings<'a> {
    pub permutation: wgpu::BufferBinding<'a>,
    pub positions_in: wgpu::BufferBinding<'a>,
    pub positions_out: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a PermutePositionsBuffers> for PermutePositionsBufferBindings<'a> {
    fn from(
        PermutePositionsBuffers {
            permutation,
            positions_in,
            positions_out,
        }: &'a PermutePositionsBuffers,
    ) -> Self {
        Self {
            permutation: permutation.as_entire_buffer_binding(),
            positions_in: positions_in.as_entire_buffer_binding(),
            positions_out: positions_out.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for PermutePositions {
    type Settings = PermutePositionsSettings;
    type Parameters = ();
    type BufferInput<'a> = PermutePositionsBufferInput<'a>;
    type Buffers = PermutePositionsBuffers;
    type BufferBindings<'a> = PermutePositionsBufferBindings<'a>;

    fn new(context: &GpuContext, Self::Settings { workgroup_size }: Self::Settings) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size > 0);
        assert!(workgroup_size.is_multiple_of(subgroup_size));

        let device = context.device();

        let label = Some("permute_positions");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[
                bind_group_layout_entry::<u32>(0, true),
                bind_group_layout_entry::<Vector4<f32>>(1, true),
                bind_group_layout_entry::<Vector4<f32>>(2, false),
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
            module: &device.create_shader_module(wgpu::include_wgsl!("permute_positions.wgsl")),
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

        Self {
            workgroup_size,
            compiled_module,
        }
    }

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput {
            permutation,
            positions,
        }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let device = context.device();
        let permutation = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("permutation"),
            contents: bytemuck::cast_slice(permutation),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let positions_in = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("positions_in"),
            contents: bytemuck::cast_slice(positions),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let positions_out = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("positions_out"),
            size: positions_in.size(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self::Buffers {
            permutation,
            positions_in,
            positions_out,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            permutation,
            positions_in,
            positions_out,
        }: Self::BufferBindings<'a>,
        (): Self::Parameters,
    ) {
        let device = context.device();

        let permutation_count = elements_in_binding::<u32>(&permutation);
        assert!(permutation_count == elements_in_binding::<Vector4<f32>>(&positions_in));
        assert!(permutation_count == elements_in_binding::<Vector4<f32>>(&positions_out));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.compiled_module.label,
            layout: &self.compiled_module.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(permutation),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(positions_in),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(positions_out),
                },
            ],
        });

        compute_pass.set_pipeline(&self.compiled_module.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroup_count = permutation_count.get().div_ceil(self.workgroup_size);
        let [x, y, z] = find_x_y_z(workgroup_count);
        compute_pass.dispatch_workgroups(x, y, z);
    }
}
