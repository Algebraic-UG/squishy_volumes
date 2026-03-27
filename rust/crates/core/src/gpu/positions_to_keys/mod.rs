// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector4;

use super::*;

#[cfg(test)]
mod test;

pub struct PositionsToKeys {
    workgroup_size: u32,
    subgroup_size: u32,
    cell_size: f32,
    compiled_module: CompiledModule,
}

impl PositionsToKeys {
    pub fn new(context: &GpuContext, workgroup_size: u32, cell_size: f32) -> Self {
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
            subgroup_size,
            cell_size,
            compiled_module,
        }
    }

    pub fn compute_in_pass(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        positions: wgpu::BufferBinding,
        keys: wgpu::BufferBinding,
        dimension: u32,
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
        let [x, y, z] = find_x_y_z(workgroup_count);

        compute_pass.set_immediates(0, bytemuck::bytes_of(&dimension));
        compute_pass.dispatch_workgroups(x, y, z);
    }
}
