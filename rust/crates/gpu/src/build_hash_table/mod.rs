// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use std::{num::NonZeroU32, sync::atomic::AtomicU32};

use nalgebra::Vector4;

use super::*;

pub struct BuildHashTable {
    build_hash_table: CompiledModule,
}

pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect_colors: Allocation,
    pub indices: Allocation,
    pub cells: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        workgroup_size: NonZeroU32,
        dispatch_limit: NonZeroU32,
        subgroup_size: NonZeroU32,
        cells: &[Vector4<i32>],
    ) -> Self {
        let (indirect_colors, _indirect_colors_batch, indices) =
            color_cells_on_cpu(workgroup_size, dispatch_limit, subgroup_size, cells);

        let indirect_colors = Allocation::new(device, "indirect_colors", &indirect_colors);
        let indices = Allocation::new(device, "indices", &indices);
        let cells = Allocation::new(device, "cells", cells);

        Self {
            indirect_colors,
            indices,
            cells,
        }
    }
}

pub struct Output {
    pub block_table: Allocation,
    pub owns: Allocation,
}

impl PipelinePart for BuildHashTable {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &GpuContext, Settings { workgroup_size }: Settings) -> Self {
        let device = context.device();
        let_compiled_module!(
            build_hash_table,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                    (Vector4::<i32>::MIN_BINDING_SIZE, false),
                    (AtomicU32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 4,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64)]
            }
        );

        Self { build_hash_table }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_colors,
            indices,
            cells,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        assert_eq!(8, indirect_colors.len::<Indirect>().get());

        let owns = context
            .allocator()?
            .allocate::<u32>("owns", cells.len::<Vector4<i32>>())?;
        let block_table = context.allocator()?.allocate::<AtomicU32>(
            "block_table",
            (self.max_table(cells.len::<Vector4<i32>>().get() as u32) as u64)
                .try_into()
                .unwrap(),
        )?;

        encoder.clear_buffer(
            block_table.buffer(),
            block_table.offset(),
            Some(block_table.size().get()),
        );

        let mut compute_pass = encoder.begin_compute_pass(self.build_hash_table.label);
        compute_pass.set_pipeline(&self.build_hash_table.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.build_hash_table,
                [
                    indirect_colors.binding(),
                    indices.binding(),
                    cells.binding(),
                    block_table.binding(),
                    owns.binding(),
                ],
            ),
            &[],
        );

        for color in 0..8u32 {
            compute_pass.set_immediates(0, bytemuck::bytes_of(&color));
            compute_pass.dispatch_workgroups_indirect(
                indirect_colors.buffer(),
                indirect_colors.offset() + Indirect::MIN_BINDING_SIZE.get() * color as u64,
            );
        }

        Ok(Output { block_table, owns })
    }
}

impl BuildHashTable {
    // control load factor to be at most 0.5
    // TODO: this is way too much for most sparsity patterns
    pub fn max_table(&self, cell_count: u32) -> u32 {
        //(cell_count * 2).next_power_of_two()
        (cell_count * 8 * 2).next_power_of_two()
    }
}
