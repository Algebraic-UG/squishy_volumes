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

pub struct AllocateBlocks {
    workgroup_size: u32,
    compiled_module: CompiledModule,
    prefix_sum: PrefixSum,
}

pub struct AllocateBlocksSettings {
    pub workgroup_size: u32,
    pub prefix_sum: PrefixSumSettings,
}

pub struct AllocateBlocksBufferInput<'a> {
    pub owns: &'a [u32],
}

pub struct AllocateBlocksBuffers {
    pub owns: wgpu::Buffer,
    pub prefix_sum: PrefixSumBuffers,
}

pub struct AllocateBlocksBufferBindings<'a> {
    pub owns: wgpu::BufferBinding<'a>,
    pub prefix_sum: PrefixSumBufferBindings<'a>,
}

impl<'a> From<&'a AllocateBlocksBuffers> for AllocateBlocksBufferBindings<'a> {
    fn from(AllocateBlocksBuffers { owns, prefix_sum }: &'a AllocateBlocksBuffers) -> Self {
        Self {
            owns: owns.as_entire_buffer_binding(),
            prefix_sum: prefix_sum.into(),
        }
    }
}

impl PipelinePart for AllocateBlocks {
    type Settings = AllocateBlocksSettings;
    type Parameters = ();
    type BufferInput<'a> = AllocateBlocksBufferInput<'a>;
    type Buffers = AllocateBlocksBuffers;
    type BufferBindings<'a> = AllocateBlocksBufferBindings<'a>;

    fn new(
        context: &GpuContext,
        Self::Settings {
            workgroup_size,
            prefix_sum,
        }: Self::Settings,
    ) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size > 0);
        assert!(workgroup_size.is_multiple_of(subgroup_size));

        let prefix_sum = PrefixSum::new(context, prefix_sum);

        let device = context.device();

        let label = Some("owns_to_pops");

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
            module: &device.create_shader_module(wgpu::include_wgsl!("owns_to_pops.wgsl")),
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
            prefix_sum,
        }
    }

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput { owns }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let device = context.device();
        let n = owns.len();

        let owns = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("owns"),
            contents: bytemuck::cast_slice(owns),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let prefix_sum = self.prefix_sum.create_buffers(
            context,
            PrefixSumBufferInput {
                numbers: &vec![0; n],
            },
        );

        Self::Buffers { owns, prefix_sum }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings { owns, prefix_sum }: Self::BufferBindings<'a>,
        _: Self::Parameters,
    ) {
        let owns_count = elements_in_binding::<u32>(&owns);
        assert!(owns_count == elements_in_binding::<u32>(&prefix_sum.numbers));

        let device = context.device();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.compiled_module.label,
            layout: &self.compiled_module.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(owns.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(prefix_sum.numbers.clone()),
                },
            ],
        });

        compute_pass.set_pipeline(&self.compiled_module.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroup_count = owns_count.get().div_ceil(self.workgroup_size) as u32;
        let [x, y, z] = find_x_y_z(workgroup_count);
        compute_pass.dispatch_workgroups(x, y, z);

        self.prefix_sum
            .compute_in_pass(context, compute_pass, prefix_sum, ());
    }
}
