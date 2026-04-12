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

pub struct Reorder {
    workgroup_size: u32,
    subgroup_size: u32,
    bit_count: u32,
    compiled_module: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct ReorderSettings {
    pub workgroup_size: u32,
    pub bit_count: u32,
}

pub struct ReorderParameters {
    pub bit_offset: u32,
}

pub struct ReorderBufferInput<'a> {
    pub keys: &'a [u32],
    pub indices: &'a [u32],
    pub prefix_sums: &'a [u32],
}

pub struct ReorderBuffers {
    pub keys: wgpu::Buffer,
    pub prefix_sums: wgpu::Buffer,
    pub indices_in: wgpu::Buffer,
    pub indices_out: wgpu::Buffer,
}

pub struct ReorderBufferBindings<'a> {
    pub keys: wgpu::BufferBinding<'a>,
    pub prefix_sums: wgpu::BufferBinding<'a>,
    pub indices_in: wgpu::BufferBinding<'a>,
    pub indices_out: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a ReorderBuffers> for ReorderBufferBindings<'a> {
    fn from(
        ReorderBuffers {
            keys,
            prefix_sums,
            indices_in,
            indices_out,
        }: &'a ReorderBuffers,
    ) -> Self {
        Self {
            keys: keys.as_entire_buffer_binding(),
            prefix_sums: prefix_sums.as_entire_buffer_binding(),
            indices_in: indices_in.as_entire_buffer_binding(),
            indices_out: indices_out.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for Reorder {
    type Settings = ReorderSettings;
    type Parameters = ReorderParameters;
    type BufferInput<'a> = ReorderBufferInput<'a>;
    type Buffers = ReorderBuffers;
    type BufferBindings<'a> = ReorderBufferBindings<'a>;

    fn new(
        context: &GpuContext,
        Self::Settings {
            workgroup_size,
            bit_count,
        }: Self::Settings,
    ) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size > 0);
        assert!(workgroup_size.is_multiple_of(subgroup_size));
        assert!(bit_count > 0);
        assert!(subgroup_size >= 2u32.pow(bit_count));

        let device = context.device();

        let label = Some("reorder");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[
                bind_group_layout_entry::<u32>(0, true),
                bind_group_layout_entry::<u32>(1, true),
                bind_group_layout_entry::<u32>(2, true),
                bind_group_layout_entry::<u32>(3, false),
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
            module: &device.create_shader_module(wgpu::include_wgsl!("reorder.wgsl")),
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[
                    ("WORKGROUP_SIZE", workgroup_size as f64),
                    ("BIT_COUNT", bit_count as f64),
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
            subgroup_size,
            bit_count,
            compiled_module,
        }
    }

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput {
            keys,
            indices,
            prefix_sums,
        }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let device = context.device();
        let keys = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("keys"),
            contents: bytemuck::cast_slice(keys),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let prefix_sums = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("prefix_sums"),
            contents: bytemuck::cast_slice(prefix_sums),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let indices_in = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indices_in"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let indices_out = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("indices_out"),
            size: indices_in.size(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self::Buffers {
            keys,
            prefix_sums,
            indices_in,
            indices_out,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            keys,
            prefix_sums,
            indices_in,
            indices_out,
        }: Self::BufferBindings<'a>,
        Self::Parameters { bit_offset }: Self::Parameters,
    ) {
        let device = context.device();

        let key_count = elements_in_binding::<u32>(&keys);
        assert!(key_count == elements_in_binding::<u32>(&indices_in));
        assert!(key_count == elements_in_binding::<u32>(&indices_out));
        let prefix_count = elements_in_binding::<u32>(&prefix_sums);
        assert!(prefix_count.get() >= self.min_prefixes(key_count.get()));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.compiled_module.label,
            layout: &self.compiled_module.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(keys.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(prefix_sums.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(indices_in.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(indices_out.clone()),
                },
            ],
        });

        compute_pass.set_pipeline(&self.compiled_module.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.set_immediates(0, bytemuck::bytes_of(&bit_offset));

        let workgroup_count = key_count.get().div_ceil(self.workgroup_size);
        let [x, y, z] = find_x_y_z_simple(u16::MAX as u32, workgroup_count);
        compute_pass.dispatch_workgroups(x, y, z);
    }
}

impl Reorder {
    pub fn min_prefixes(&self, key_count: u32) -> u32 {
        let subgroups_per_workgroup = self.workgroup_size / self.subgroup_size;
        let workgroup_count = key_count.div_ceil(self.workgroup_size);
        let actual_workgroup_count = find_x_y_z_simple(u16::MAX as u32, workgroup_count)
            .into_iter()
            .product::<u32>();
        actual_workgroup_count * subgroups_per_workgroup * 2u32.pow(self.bit_count)
    }
}
