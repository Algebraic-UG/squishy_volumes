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

pub struct CellsToColorkeys {
    workgroup_size: u32,
    compiled_module: CompiledModule,
}
pub struct CellsToColorkeysSettings {
    pub workgroup_size: u32,
}
pub struct CellsToColorkeysBufferInput<'a> {
    pub cells: &'a [Vector4<i32>],
}
pub struct CellsToColorkeysBuffers {
    pub cells: wgpu::Buffer,
    pub keys: wgpu::Buffer,
}
pub struct CellsToColorkeysBufferBindings<'a> {
    pub cells: wgpu::BufferBinding<'a>,
    pub keys: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a CellsToColorkeysBuffers> for CellsToColorkeysBufferBindings<'a> {
    fn from(CellsToColorkeysBuffers { cells, keys }: &'a CellsToColorkeysBuffers) -> Self {
        Self {
            cells: cells.as_entire_buffer_binding(),
            keys: keys.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for CellsToColorkeys {
    type Settings = CellsToColorkeysSettings;
    type Parameters = ();
    type BufferInput<'a> = CellsToColorkeysBufferInput<'a>;
    type Buffers = CellsToColorkeysBuffers;
    type BufferBindings<'a> = CellsToColorkeysBufferBindings<'a>;

    fn new(context: &GpuContext, Self::Settings { workgroup_size }: Self::Settings) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size > 0);
        assert!(workgroup_size.is_multiple_of(subgroup_size));

        let device = context.device();

        let label = Some("cells");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[
                bind_group_layout_entry::<Vector4<i32>>(0, true),
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
            module: &device.create_shader_module(wgpu::include_wgsl!("cells_to_colorkeys.wgsl")),
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
        Self::BufferInput { cells }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let device = context.device();
        let n = cells.len();

        let cells = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cells"),
            contents: bytemuck::cast_slice(cells),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let keys = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("keys"),
            size: n as u64 * u32::MIN_BINDING_SIZE.get(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self::Buffers { cells, keys }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings { cells, keys }: &mut Self::BufferBindings<'a>,
        _: &mut Self::Parameters,
    ) {
        let cell_count = elements_in_binding::<Vector4<i32>>(cells);
        assert!(cell_count == elements_in_binding::<u32>(keys));

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
                    resource: wgpu::BindingResource::Buffer(keys.clone()),
                },
            ],
        });

        compute_pass.set_pipeline(&self.compiled_module.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroup_count = cell_count.get().div_ceil(self.workgroup_size) as u32;
        let [x, y, z] = find_x_y_z(workgroup_count);

        compute_pass.dispatch_workgroups(x, y, z);
    }
}
