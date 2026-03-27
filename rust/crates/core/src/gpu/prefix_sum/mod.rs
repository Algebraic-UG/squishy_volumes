// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU64;

use super::*;

#[cfg(test)]
mod test;

pub struct PrefixSum {
    workgroup_size: u32,
    subgroup_size: u32,
    build_levels: CompiledModule,
    fill_final: CompiledModule,
}

impl PrefixSum {
    pub fn new(context: &GpuContext, workgroup_size: u32) -> Self {
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

    pub fn compute_in_pass(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        input: wgpu::BufferBinding,
        output: wgpu::BufferBinding,
    ) {
        let element_size = NonZeroU64::new(4).unwrap();
        let element_count = elements_in_binding(&element_size, &input);
        assert!(element_count == elements_in_binding(&element_size, &output));

        let element_count = element_count.get();
        let max_level = (element_count * self.subgroup_size - 1).ilog(self.subgroup_size);

        let device = context.device();
        let bind_group_build_levels = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.build_levels.label,
            layout: &self.build_levels.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(input.clone()),
            }],
        });
        let bind_group_fill_final = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.fill_final.label,
            layout: &self.fill_final.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(input),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(output),
                },
            ],
        });

        compute_pass.set_pipeline(&self.build_levels.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group_build_levels, &[]);

        for level in 0..max_level {
            let stride = self.subgroup_size.pow(level);
            let element_count = element_count.div_ceil(self.subgroup_size.pow(level));

            let workgroup_count = element_count.div_ceil(self.workgroup_size) as u32;
            let [x, y, z] = find_x_y_z(workgroup_count);
            tracing::info!(level, element_count, x, y, z);

            compute_pass.set_immediates(0, bytemuck::bytes_of(&stride));
            compute_pass.dispatch_workgroups(x, y, z);
        }

        compute_pass.set_pipeline(&self.fill_final.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group_fill_final, &[]);
        compute_pass.set_immediates(0, bytemuck::bytes_of(&max_level));

        let workgroup_count = element_count.div_ceil(self.workgroup_size) as u32;
        let [x, y, z] = find_x_y_z(workgroup_count);
        compute_pass.dispatch_workgroups(x, y, z);
    }
}
