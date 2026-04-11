// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use super::*;

pub struct GenerateIndices {
    workgroup_size: u32,
    compiled_module: CompiledModule,
}

pub struct GenerateIndicesSettings {
    pub workgroup_size: u32,
}

pub struct GenerateIndicesBufferInput<'a> {
    pub count: &'a u32,
}

pub struct GenerateIndicesBuffers {
    pub indices: wgpu::Buffer,
}

pub struct GenerateIndicesBufferBindings<'a> {
    pub indices: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a GenerateIndicesBuffers> for GenerateIndicesBufferBindings<'a> {
    fn from(GenerateIndicesBuffers { indices }: &'a GenerateIndicesBuffers) -> Self {
        Self {
            indices: indices.as_entire_buffer_binding(),
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
            entries: &[bind_group_layout_entry::<u32>(0, false)],
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

        Self {
            workgroup_size,
            compiled_module,
        }
    }

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput { count }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let_buffer!(context.device(), indices<u32>(*count, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));

        Self::Buffers { indices }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings { indices }: Self::BufferBindings<'a>,
        _: Self::Parameters,
    ) {
        let count = elements_in_binding::<u32>(&indices);

        let device = context.device();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.compiled_module.label,
            layout: &self.compiled_module.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(indices),
            }],
        });

        compute_pass.set_pipeline(&self.compiled_module.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroup_count = count.get().div_ceil(self.workgroup_size) as u32;
        let [x, y, z] = find_x_y_z(workgroup_count);

        compute_pass.dispatch_workgroups(x, y, z);
    }
}
