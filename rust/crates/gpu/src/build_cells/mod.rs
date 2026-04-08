// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use nalgebra::Vector4;
use wgpu::util::DeviceExt as _;

use super::*;

pub struct BuildCells {
    workgroup_size: u32,
    compiled_module: CompiledModule,
}

pub struct BuildCellsSettings {
    pub workgroup_size: u32,
    pub cell_size: f32,
}

pub struct BuildCellsBufferInput<'a> {
    pub positions: &'a [Vector4<f32>],
    pub prefixed_boundaries: &'a [u32],
}

pub struct BuildCellsBuffers {
    pub positions: wgpu::Buffer,
    pub prefixed_boundaries: wgpu::Buffer,
    pub cells: wgpu::Buffer,
    pub index_ranges: wgpu::Buffer,
}

pub struct BuildCellsBufferBindings<'a> {
    pub positions: wgpu::BufferBinding<'a>,
    pub prefixed_boundaries: wgpu::BufferBinding<'a>,
    pub cells: wgpu::BufferBinding<'a>,
    pub index_ranges: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a BuildCellsBuffers> for BuildCellsBufferBindings<'a> {
    fn from(
        BuildCellsBuffers {
            positions,
            prefixed_boundaries,
            cells,
            index_ranges,
        }: &'a BuildCellsBuffers,
    ) -> Self {
        Self {
            positions: positions.as_entire_buffer_binding(),
            prefixed_boundaries: prefixed_boundaries.as_entire_buffer_binding(),
            cells: cells.as_entire_buffer_binding(),
            index_ranges: index_ranges.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for BuildCells {
    type Settings = BuildCellsSettings;
    type Parameters = ();
    type BufferInput<'a> = BuildCellsBufferInput<'a>;
    type Buffers = BuildCellsBuffers;
    type BufferBindings<'a> = BuildCellsBufferBindings<'a>;

    fn new(
        context: &GpuContext,
        Self::Settings {
            workgroup_size,
            cell_size,
        }: Self::Settings,
    ) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size > 0);
        assert!(workgroup_size.is_multiple_of(subgroup_size));
        let device = context.device();

        let label = Some("build_cells");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[
                bind_group_layout_entry::<Vector4<f32>>(0, true),
                bind_group_layout_entry::<u32>(1, true),
                bind_group_layout_entry::<Vector4<i32>>(2, false),
                bind_group_layout_entry::<u32>(3, false),
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
            module: &device.create_shader_module(wgpu::include_wgsl!("build_cells.wgsl")),
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[
                    ("WORKGROUP_SIZE", workgroup_size as f64),
                    ("CELL_SIZE", cell_size as f64),
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

        Self {
            workgroup_size,
            compiled_module,
        }
    }

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput {
            positions,
            prefixed_boundaries,
        }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        assert_eq!(positions.len(), prefixed_boundaries.len());
        let device = context.device();

        let positions = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("positions"),
            contents: bytemuck::cast_slice(positions),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let prefixed_boundaries = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("prefixed_boundaries"),
            contents: bytemuck::cast_slice(prefixed_boundaries),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let cells = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cells"),
            size: positions.size(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let index_ranges = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index_ranges"),
            size: prefixed_boundaries.size(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self::Buffers {
            positions,
            prefixed_boundaries,
            cells,
            index_ranges,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            positions,
            prefixed_boundaries,
            cells,
            index_ranges,
        }: Self::BufferBindings<'a>,
        _: Self::Parameters,
    ) {
        let position_count = elements_in_binding::<Vector4<f32>>(&positions);
        assert!(position_count == elements_in_binding::<u32>(&prefixed_boundaries));
        assert!(position_count == elements_in_binding::<Vector4<i32>>(&cells));
        assert!(position_count == elements_in_binding::<u32>(&index_ranges));

        let device = context.device();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.compiled_module.label,
            layout: &self.compiled_module.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(positions.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(prefixed_boundaries.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(cells.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(index_ranges.clone()),
                },
            ],
        });

        compute_pass.set_pipeline(&self.compiled_module.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroup_count = position_count.get().div_ceil(self.workgroup_size) as u32;
        let [x, y, z] = find_x_y_z(workgroup_count);

        compute_pass.dispatch_workgroups(x, y, z);
    }
}
