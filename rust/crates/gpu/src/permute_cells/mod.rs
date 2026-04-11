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

pub struct PermuteCells {
    workgroup_size: u32,
    compiled_module: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct PermuteCellsSettings {
    pub workgroup_size: u32,
}

pub struct PermuteCellsBufferInput<'a> {
    pub permutation: &'a [u32],
    pub cells: &'a [Vector4<i32>],
}

pub struct PermuteCellsBuffers {
    pub permutation: wgpu::Buffer,
    pub cells_in: wgpu::Buffer,
    pub cells_out: wgpu::Buffer,
}

pub struct PermuteCellsBufferBindings<'a> {
    pub permutation: wgpu::BufferBinding<'a>,
    pub cells_in: wgpu::BufferBinding<'a>,
    pub cells_out: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a PermuteCellsBuffers> for PermuteCellsBufferBindings<'a> {
    fn from(
        PermuteCellsBuffers {
            permutation,
            cells_in,
            cells_out,
        }: &'a PermuteCellsBuffers,
    ) -> Self {
        Self {
            permutation: permutation.as_entire_buffer_binding(),
            cells_in: cells_in.as_entire_buffer_binding(),
            cells_out: cells_out.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for PermuteCells {
    type Settings = PermuteCellsSettings;
    type Parameters = ();
    type BufferInput<'a> = PermuteCellsBufferInput<'a>;
    type Buffers = PermuteCellsBuffers;
    type BufferBindings<'a> = PermuteCellsBufferBindings<'a>;

    fn new(context: &GpuContext, Self::Settings { workgroup_size }: Self::Settings) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size > 0);
        assert!(workgroup_size.is_multiple_of(subgroup_size));

        let device = context.device();

        let label = Some("permute_cells");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[
                bind_group_layout_entry::<u32>(0, true),
                bind_group_layout_entry::<Vector4<i32>>(1, true),
                bind_group_layout_entry::<Vector4<i32>>(2, false),
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
            module: &device.create_shader_module(wgpu::include_wgsl!("permute_cells.wgsl")),
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
        Self::BufferInput { permutation, cells }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let device = context.device();
        let permutation = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("permutation"),
            contents: bytemuck::cast_slice(permutation),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let cells_in = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cells_in"),
            contents: bytemuck::cast_slice(cells),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let cells_out = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cells_out"),
            size: cells_in.size(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self::Buffers {
            permutation,
            cells_in,
            cells_out,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            permutation,
            cells_in,
            cells_out,
        }: Self::BufferBindings<'a>,
        (): Self::Parameters,
    ) {
        let device = context.device();

        let permutation_count = elements_in_binding::<u32>(&permutation);
        assert!(permutation_count == elements_in_binding::<Vector4<i32>>(&cells_in));
        assert!(permutation_count == elements_in_binding::<Vector4<i32>>(&cells_out));

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
                    resource: wgpu::BindingResource::Buffer(cells_in),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(cells_out),
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
