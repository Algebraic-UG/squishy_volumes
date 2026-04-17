// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use std::num::NonZeroU32;

use nalgebra::Vector4;
use wgpu::util::DeviceExt as _;

use super::*;

pub struct ColorCells {
    workgroup_size: u32,
    subgroup_size: u32,
    dispatch_limit: u32,
    count_colors: CompiledModule,
    prefix_sum: PrefixSum,
    finalize_colors: CompiledModule,
    recycle_to_indirect: RecycleToIndirect,
}

pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub cell_ids_in: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
        cell_ids: &[Vector4<i32>],
    ) -> Self {
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: cell_ids.len() as u32,
        });
        let indirect = Allocation::new(device, "indirect", &[indirect]);
        let cell_ids_in = Allocation::new(device, "cell_ids_in", cell_ids);
        Self {
            indirect,
            cell_ids_in,
        }
    }
}

pub struct Output {
    pub indirect_colors: Allocation,
    pub permutation: Allocation,
    pub cell_ids_out: Allocation,
}

impl PipelinePart for ColorCells {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
    ) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size.get().is_multiple_of(subgroup_size));

        // one per dimension
        let bit_count = 3;
        assert!(subgroup_size >= 2u32.pow(bit_count));

        let device = context.device();

        let_compiled_module!(
            count_colors,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (Vector4::<i32>::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64)]
            }
        );

        let_compiled_module!(
            finalize_colors,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                    (Vector4::<i32>::MIN_BINDING_SIZE, false),
                    (Vector4::<i32>::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64)]
            }
        );

        let prefix_sum = PrefixSum::new(
            context,
            prefix_sum::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );

        let recycle_to_indirect = RecycleToIndirect::new(
            context,
            recycle_to_indirect::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );

        Self {
            count_colors,
            prefix_sum,
            finalize_colors,
            recycle_to_indirect,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            cell_ids_in,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let color_counts_len =
            (self.min_color_counts_len(cell_ids_in.len::<Vector4<i32>>().get() as u32) as u64)
                .try_into()
                .unwrap();
        let color_counts = context
            .allocator()?
            .allocate::<u32>("color_counts", color_counts_len)?;

        let mut compute_pass = encoder.begin_compute_pass(self.count_colors.label);
        compute_pass.set_pipeline(&self.count_colors.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.count_colors,
                [
                    indirect.binding(),
                    cell_ids_in.binding(),
                    color_counts.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());
        drop(compute_pass);

        self.prefix_sum.record(
            context,
            encoder
            prefix_sum::Input {
                indirect,
                numbers: todo!(),
            },
            prefix_sum::Parameters,
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

impl ColorCells {
    pub fn min_color_counts_len(&self, len: u32) -> u32 {
        let subgroups_per_workgroup = self.workgroup_size / self.subgroup_size;
        let actual_workgroup_count = Indirect::new(IndirectSettings {
            workgroup_size: self.workgroup_size.try_into().unwrap(),
            dispatch_limit: self.dispatch_limit.try_into().unwrap(),
            len,
        })
        .workgroup_count();
        actual_workgroup_count * subgroups_per_workgroup * 2u32.pow(3)
    }
}
