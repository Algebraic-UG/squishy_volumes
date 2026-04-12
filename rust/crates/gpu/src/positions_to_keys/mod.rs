// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector4;
use wgpu::util::DeviceExt as _;

use super::*;

#[cfg(test)]
mod test;

pub struct PositionsToKeys {
    workgroup_size: u32,
    compiled_module: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct PositionsToKeysSettings {
    pub workgroup_size: u32,
    pub cell_size: f32,
}

pub struct PositionsToKeysParameters {
    pub dimension: u32,
}

pub struct PositionsToKeysBufferInput<'a> {
    pub positions: &'a [Vector4<f32>],
}

pub struct PositionsToKeysBuffers {
    pub positions: wgpu::Buffer,
    pub keys: wgpu::Buffer,
}

pub struct PositionsToKeysBufferBindings<'a> {
    pub positions: wgpu::BufferBinding<'a>,
    pub keys: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a PositionsToKeysBuffers> for PositionsToKeysBufferBindings<'a> {
    fn from(PositionsToKeysBuffers { positions, keys }: &'a PositionsToKeysBuffers) -> Self {
        Self {
            positions: positions.as_entire_buffer_binding(),
            keys: keys.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for PositionsToKeys {
    type Settings = PositionsToKeysSettings;
    type Parameters = PositionsToKeysParameters;
    type BufferInput<'a> = PositionsToKeysBufferInput<'a>;
    type Buffers = PositionsToKeysBuffers;
    type BufferBindings<'a> = PositionsToKeysBufferBindings<'a>;

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

        let label = Some("positions_to_keys");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[
                bind_group_layout_entry::<Vector4<f32>>(0, true),
                bind_group_layout_entry::<u32>(1, false),
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
            module: &device.create_shader_module(wgpu::include_wgsl!("positions_to_keys.wgsl")),
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
        Self::BufferInput { positions }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let device = context.device();
        let n = positions.len();

        let positions = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("positions"),
            contents: bytemuck::cast_slice(positions),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let keys = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("keys"),
            size: n as u64 * u32::MIN_BINDING_SIZE.get(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self::Buffers { positions, keys }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings { positions, keys }: Self::BufferBindings<'a>,
        Self::Parameters { dimension }: Self::Parameters,
    ) {
        assert!(dimension < 3);

        let position_count = elements_in_binding::<Vector4<f32>>(&positions);
        assert!(position_count == elements_in_binding::<u32>(&keys));

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
                    resource: wgpu::BindingResource::Buffer(keys.clone()),
                },
            ],
        });

        compute_pass.set_pipeline(&self.compiled_module.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroup_count = position_count.get().div_ceil(self.workgroup_size) as u32;
        let [x, y, z] = find_x_y_z_simple(u16::MAX as u32, workgroup_count);

        compute_pass.set_immediates(0, bytemuck::bytes_of(&dimension));
        compute_pass.dispatch_workgroups(x, y, z);
    }
}
