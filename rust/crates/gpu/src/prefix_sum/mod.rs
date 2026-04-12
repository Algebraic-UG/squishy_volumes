// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use wgpu::util::DeviceExt as _;

use super::*;

#[cfg(test)]
mod test;

pub struct PrefixSum {
    workgroup_size: u32,
    subgroup_size: u32,
    build_levels: CompiledModule,
    fill_final: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct PrefixSumSettings {
    pub workgroup_size: u32,
}

pub struct PrefixSumBufferInput<'a> {
    pub numbers: &'a [u32],
}

pub struct PrefixSumBuffers {
    pub numbers: wgpu::Buffer,
    pub prefix_sums: wgpu::Buffer,
}

pub struct PrefixSumBufferBindings<'a> {
    pub numbers: wgpu::BufferBinding<'a>,
    pub prefix_sums: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a PrefixSumBuffers> for PrefixSumBufferBindings<'a> {
    fn from(
        PrefixSumBuffers {
            numbers,
            prefix_sums,
        }: &'a PrefixSumBuffers,
    ) -> Self {
        Self {
            numbers: numbers.as_entire_buffer_binding(),
            prefix_sums: prefix_sums.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for PrefixSum {
    type Settings = PrefixSumSettings;
    type Parameters = ();
    type BufferInput<'a> = PrefixSumBufferInput<'a>;
    type Buffers = PrefixSumBuffers;
    type BufferBindings<'a> = PrefixSumBufferBindings<'a>;

    fn new(context: &GpuContext, Self::Settings { workgroup_size }: Self::Settings) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size > 0);
        assert!(workgroup_size.is_multiple_of(subgroup_size));

        let device = context.device();

        let build_levels = {
            let label = Some("build_levels");

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label,
                    entries: &[bind_group_layout_entry::<u32>(0, false)],
                });

            let compute_pipeline =
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label,
                    layout: Some(
                        &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label,
                            bind_group_layouts: &[Some(&bind_group_layout)],
                            immediate_size: 4,
                        }),
                    ),
                    module: &device.create_shader_module(wgpu::include_wgsl!("build_levels.wgsl")),
                    entry_point: Some("main"),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &[("WORKGROUP_SIZE", workgroup_size as f64)],
                        ..Default::default()
                    },
                    cache: None,
                });

            CompiledModule {
                label,
                bind_group_layout,
                compute_pipeline,
            }
        };

        let fill_final = {
            let label = Some("fill_final");

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label,
                    entries: &[
                        bind_group_layout_entry::<u32>(0, true),
                        bind_group_layout_entry::<u32>(1, false),
                    ],
                });

            let compute_pipeline =
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label,
                    layout: Some(
                        &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label,
                            bind_group_layouts: &[Some(&bind_group_layout)],
                            immediate_size: 4,
                        }),
                    ),
                    module: &device.create_shader_module(wgpu::include_wgsl!("fill_final.wgsl")),
                    entry_point: Some("main"),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &[("WORKGROUP_SIZE", workgroup_size as f64)],
                        ..Default::default()
                    },
                    cache: None,
                });

            CompiledModule {
                label,
                bind_group_layout,
                compute_pipeline,
            }
        };

        Self {
            workgroup_size,
            subgroup_size,
            build_levels,
            fill_final,
        }
    }

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput { numbers }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let device = context.device();
        let numbers = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("numbers"),
            contents: bytemuck::cast_slice(numbers),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let prefix_sums = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("prefix_sums"),
            size: numbers.size(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self::Buffers {
            numbers,
            prefix_sums,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            numbers,
            prefix_sums,
        }: Self::BufferBindings<'a>,
        _: Self::Parameters,
    ) {
        let element_count = elements_in_binding::<u32>(&numbers);
        assert!(element_count == elements_in_binding::<u32>(&prefix_sums));

        let element_count = element_count.get();
        let max_level = (element_count * self.subgroup_size - 1).ilog(self.subgroup_size);

        let device = context.device();
        let bind_group_build_levels = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.build_levels.label,
            layout: &self.build_levels.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(numbers.clone()),
            }],
        });
        let bind_group_fill_final = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.fill_final.label,
            layout: &self.fill_final.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(numbers.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(prefix_sums.clone()),
                },
            ],
        });

        compute_pass.set_pipeline(&self.build_levels.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group_build_levels, &[]);

        for level in 0..max_level {
            let stride = self.subgroup_size.pow(level);
            let element_count = element_count.div_ceil(self.subgroup_size.pow(level));

            let workgroup_count = element_count.div_ceil(self.workgroup_size) as u32;
            let [x, y, z] = find_x_y_z_simple(u16::MAX as u32, workgroup_count);
            tracing::info!(level, element_count, x, y, z);

            compute_pass.set_immediates(0, bytemuck::bytes_of(&stride));
            compute_pass.dispatch_workgroups(x, y, z);
        }

        compute_pass.set_pipeline(&self.fill_final.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group_fill_final, &[]);
        compute_pass.set_immediates(0, bytemuck::bytes_of(&max_level));

        let workgroup_count = element_count.div_ceil(self.workgroup_size) as u32;
        let [x, y, z] = find_x_y_z_simple(u16::MAX as u32, workgroup_count);
        compute_pass.dispatch_workgroups(x, y, z);
    }
}
