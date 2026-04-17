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

pub struct ColorCells2 {
    workgroup_size: u32,
    subgroup_size: u32,
    dispatch_limit: u32,
    count_colors: CompiledModule,
    prefix_sum: PrefixSum,
    finalize_colors: CompiledModule,
    recycle_to_indirect: RecycleToIndirect,
}

pub struct ColorCells2Settings {
    pub workgroup_size: u32,
    pub dispatch_limit: u32,
}

pub struct ColorCells2BufferInput<'a> {
    pub cells: &'a [Vector4<i32>],
    pub limits: &'a [u32],
    pub indirect: &'a [u32],
}

pub struct ColorCells2Buffers {
    pub cells_in: wgpu::Buffer,
    pub cells_out: wgpu::Buffer,
    pub limits: wgpu::Buffer,
    pub indirect: wgpu::Buffer,
    pub counts: wgpu::Buffer,
    pub prefix_sums: wgpu::Buffer,
}

pub struct ColorCells2BufferBindings<'a> {
    pub cells_in: wgpu::BufferBinding<'a>,
    pub cells_out: wgpu::BufferBinding<'a>,
    pub limits: wgpu::BufferBinding<'a>,
    pub indirect: wgpu::BufferBinding<'a>,
    pub counts: wgpu::BufferBinding<'a>,
    pub prefix_sums: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a ColorCells2Buffers> for ColorCells2BufferBindings<'a> {
    fn from(
        ColorCells2Buffers {
            cells_in,
            cells_out,
            indirect,
            limits,
            counts,
            prefix_sums,
        }: &'a ColorCells2Buffers,
    ) -> Self {
        Self {
            cells_in: cells_in.as_entire_buffer_binding(),
            cells_out: cells_out.as_entire_buffer_binding(),
            limits: limits.as_entire_buffer_binding(),
            indirect: indirect.as_entire_buffer_binding(),
            counts: counts.as_entire_buffer_binding(),
            prefix_sums: prefix_sums.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for ColorCells2 {
    type Settings = ColorCells2Settings;
    type Parameters = ();
    type BufferInput<'a> = ColorCells2BufferInput<'a>;
    type Buffers = ColorCells2Buffers;
    type BufferBindings<'a> = ColorCells2BufferBindings<'a>;

    fn new(
        context: &GpuContext,
        Self::Settings {
            workgroup_size,
            dispatch_limit,
        }: Self::Settings,
    ) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size > 0);
        assert!(workgroup_size.is_multiple_of(subgroup_size));

        // one per dimension
        let bit_count = 3;
        assert!(subgroup_size >= 2u32.pow(bit_count));

        let device = context.device();

        let count_colors = {
            let label = Some("count_colors");
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label,
                    entries: &[
                        bind_group_layout_entry::<u32>(0, true),
                        bind_group_layout_entry::<Vector4<i32>>(1, true),
                        bind_group_layout_entry::<u32>(2, false),
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
                    module: &device.create_shader_module(wgpu::include_wgsl!("count_colors.wgsl")),
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

        let prefix_sum = PrefixSum::new(context, PrefixSumSettings { workgroup_size });

        let finalize_colors = {
            let label = Some("finalize_colors");
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label,
                    entries: &[
                        bind_group_layout_entry::<u32>(0, true),
                        bind_group_layout_entry::<u32>(1, true),
                        bind_group_layout_entry::<Vector4<i32>>(2, true),
                        bind_group_layout_entry::<Vector4<i32>>(3, false),
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
                        .create_shader_module(wgpu::include_wgsl!("finalize_colors.wgsl")),
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

        let recycle_to_indirect = RecycleToIndirect::new(
            context,
            RecycleToIndirectSettings {
                workgroup_size,
                dispatch_limit,
            },
        );

        Self {
            workgroup_size,
            subgroup_size,
            dispatch_limit,
            count_colors,
            prefix_sum,
            finalize_colors,
            recycle_to_indirect,
        }
    }

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput {
            cells,
            limits,
            indirect,
        }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let device = context.device();
        let n = cells.len();
        let count_n =
            self.min_counts_and_prefixes_given_indirect(&indirect[0..3].try_into().unwrap());

        let cells_in = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cells_in"),
            contents: bytemuck::cast_slice(cells),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let limits = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("limits"),
            contents: bytemuck::cast_slice(limits),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let indirect = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indirect"),
            contents: bytemuck::cast_slice(indirect),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::INDIRECT,
        });

        let_buffer!(device, cells_out<Vector4<i32>>(n, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));

        let_buffer!(device, counts<u32>(count_n, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));
        let_buffer!(device, prefix_sums<u32>(count_n, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC));

        Self::Buffers {
            cells_in,
            cells_out,
            limits,
            indirect,
            counts,
            prefix_sums,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            cells_in,
            cells_out,
            limits,
            indirect,
            counts,
            prefix_sums,
        }: Self::BufferBindings<'a>,
        _: Self::Parameters,
    ) {
        let device = context.device();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.count_colors.label,
            layout: &self.count_colors.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(limits.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(cells_in.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(counts.clone()),
                },
            ],
        });
        compute_pass.set_pipeline(&self.count_colors.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups_indirect(indirect.buffer, 0);

        self.prefix_sum.compute_in_pass(
            context,
            compute_pass,
            PrefixSumBufferBindings {
                numbers: counts.clone(),
                prefix_sums: prefix_sums.clone(),
            },
            (),
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.finalize_colors.label,
            layout: &self.finalize_colors.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(limits.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(prefix_sums.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(cells_in.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(cells_out.clone()),
                },
            ],
        });
        compute_pass.set_pipeline(&self.finalize_colors.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups_indirect(indirect.buffer, 0);

        self.recycle_to_indirect.compute_in_pass(
            context,
            compute_pass,
            RecycleToIndirectBufferBindings {
                indirect,
                limits,
                prefix_sums,
            },
            (),
        );
    }
}

impl ColorCells2 {
    pub fn min_counts_and_prefixes(&self, cell_count: u32) -> u32 {
        let workgroup_count = cell_count.div_ceil(self.workgroup_size);
        self.min_counts_and_prefixes_given_indirect(&find_x_y_z_simple(
            self.dispatch_limit,
            workgroup_count,
        ))
    }

    pub fn min_counts_and_prefixes_given_indirect(&self, indirect: &[u32; 3]) -> u32 {
        let subgroups_per_workgroup = self.workgroup_size / self.subgroup_size;
        let actual_workgroup_count = indirect.iter().product::<u32>();
        actual_workgroup_count * subgroups_per_workgroup * 2u32.pow(3)
    }
}
