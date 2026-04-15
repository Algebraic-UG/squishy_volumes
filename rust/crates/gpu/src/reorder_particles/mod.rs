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

pub struct ReorderParticles {
    index_ranges_to_counts: CompiledModule,
    prefix_sum: PrefixSum,
    move_cells: CompiledModule,
}

pub struct ReorderParticlesSettings {
    pub workgroup_size: u32,
    pub prefix_sum: PrefixSumSettings,
}

pub struct ReorderParticlesBufferInput<'a> {
    pub limits: &'a [u32],
    pub indirect: &'a [u32],
    pub cell_indices: &'a [u32],
    pub index_ranges: &'a [u32],
    pub positions: &'a [Vector4<f32>],
}

pub struct ReorderParticlesBuffers {
    pub limits: wgpu::Buffer,
    pub indirect: wgpu::Buffer,
    pub cell_indices: wgpu::Buffer,
    pub index_ranges: wgpu::Buffer,
    pub counts: wgpu::Buffer,
    pub offsets: wgpu::Buffer,
    pub positions_in: wgpu::Buffer,
    pub positions_out: wgpu::Buffer,
}

pub struct ReorderParticlesBufferBindings<'a> {
    pub limits: wgpu::BufferBinding<'a>,
    pub indirect: wgpu::BufferBinding<'a>,
    pub cell_indices: wgpu::BufferBinding<'a>,
    pub index_ranges: wgpu::BufferBinding<'a>,
    pub counts: wgpu::BufferBinding<'a>,
    pub offsets: wgpu::BufferBinding<'a>,
    pub positions_in: wgpu::BufferBinding<'a>,
    pub positions_out: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a ReorderParticlesBuffers> for ReorderParticlesBufferBindings<'a> {
    fn from(
        ReorderParticlesBuffers {
            limits,
            indirect,
            cell_indices,
            index_ranges,
            counts,
            offsets,
            positions_in,
            positions_out,
        }: &'a ReorderParticlesBuffers,
    ) -> Self {
        Self {
            limits: limits.as_entire_buffer_binding(),
            indirect: indirect.as_entire_buffer_binding(),
            cell_indices: cell_indices.as_entire_buffer_binding(),
            index_ranges: index_ranges.as_entire_buffer_binding(),
            counts: counts.as_entire_buffer_binding(),
            offsets: offsets.as_entire_buffer_binding(),
            positions_in: positions_in.as_entire_buffer_binding(),
            positions_out: positions_out.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for ReorderParticles {
    type Settings = ReorderParticlesSettings;
    type Parameters = ();
    type BufferInput<'a> = ReorderParticlesBufferInput<'a>;
    type Buffers = ReorderParticlesBuffers;
    type BufferBindings<'a> = ReorderParticlesBufferBindings<'a>;

    fn new(
        context: &GpuContext,
        Self::Settings {
            workgroup_size,
            prefix_sum,
        }: Self::Settings,
    ) -> Self {
        assert!(workgroup_size > 0);

        let device = context.device();
        let index_ranges_to_counts = {
            let label = Some("index_ranges_to_counts");
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label,
                    entries: &[
                        bind_group_layout_entry::<u32>(0, true),
                        bind_group_layout_entry::<u32>(1, true),
                        bind_group_layout_entry::<u32>(2, true),
                        bind_group_layout_entry::<u32>(3, false),
                    ],
                });

            let compute_pipeline =
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label,
                    layout: Some(
                        &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label,
                            bind_group_layouts: &[Some(&bind_group_layout)],
                            ..Default::default()
                        }),
                    ),
                    module: &device
                        .create_shader_module(wgpu::include_wgsl!("index_ranges_to_counts.wgsl")),
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

        let move_cells = {
            let label = Some("move_cells");
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label,
                    entries: &[
                        bind_group_layout_entry::<u32>(0, true),
                        bind_group_layout_entry::<u32>(1, true),
                        bind_group_layout_entry::<u32>(2, true),
                        bind_group_layout_entry::<u32>(3, true),
                        bind_group_layout_entry::<Vector4<f32>>(4, true),
                        bind_group_layout_entry::<Vector4<f32>>(5, false),
                    ],
                });

            let compute_pipeline =
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label,
                    layout: Some(
                        &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label,
                            bind_group_layouts: &[Some(&bind_group_layout)],
                            ..Default::default()
                        }),
                    ),
                    module: &device.create_shader_module(wgpu::include_wgsl!("move_cells.wgsl")),
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
            index_ranges_to_counts,
            prefix_sum: PrefixSum::new(context, prefix_sum),
            move_cells,
        }
    }

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput {
            limits,
            indirect,
            cell_indices,
            index_ranges,
            positions,
        }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let device = context.device();

        let cell_n = cell_indices.len();
        let particle_n = positions.len();
        assert_eq!(cell_indices.len(), index_ranges.len());

        let limits = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("limits"),
            contents: bytemuck::cast_slice(limits),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let indirect = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indirect"),
            contents: bytemuck::cast_slice(indirect),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
        });

        let cell_indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cell_indices"),
            contents: bytemuck::cast_slice(cell_indices),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let index_ranges = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_ranges"),
            contents: bytemuck::cast_slice(index_ranges),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let positions_in = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("positions_in"),
            contents: bytemuck::cast_slice(positions),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let_buffer!(device, counts<u32>(cell_n, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));
        let_buffer!(device, offsets<u32>(cell_n, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));
        let_buffer!(device, positions_out<Vector4<f32>>(particle_n, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));

        Self::Buffers {
            limits,
            indirect,
            cell_indices,
            index_ranges,
            counts,
            offsets,
            positions_in,
            positions_out,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            limits,
            indirect,
            cell_indices,
            index_ranges,
            counts,
            offsets,
            positions_in,
            positions_out,
        }: Self::BufferBindings<'a>,
        _: Self::Parameters,
    ) {
        let device = context.device();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.index_ranges_to_counts.label,
            layout: &self.index_ranges_to_counts.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(limits.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(cell_indices.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(index_ranges.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(counts.clone()),
                },
            ],
        });
        compute_pass.set_pipeline(&self.index_ranges_to_counts.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups_indirect(indirect.buffer, 0);

        self.prefix_sum.compute_in_pass(
            context,
            compute_pass,
            PrefixSumBufferBindings {
                numbers: counts,
                prefix_sums: offsets.clone(),
            },
            (),
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.move_cells.label,
            layout: &self.move_cells.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(limits),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(cell_indices),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(index_ranges),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(offsets),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(positions_in),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Buffer(positions_out),
                },
            ],
        });
        compute_pass.set_pipeline(&self.move_cells.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups_indirect(indirect.buffer, 0);
    }
}
