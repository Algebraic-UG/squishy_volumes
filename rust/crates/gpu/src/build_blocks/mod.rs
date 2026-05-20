// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{num::NonZeroU32, sync::atomic::AtomicU32};

use nalgebra::Vector4;

#[cfg(test)]
mod test;

use super::*;

pub struct BuildBlocks {
    build_hash_table_from_cells: BuildHashTableFromCells,
    allocate_blocks: AllocateBlocks,
    offsets_to_indirect: OffsetsToIndirect,
    build_blocks: CompiledModule,
    build_hash_table_from_blocks: CompiledModule,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect_cells: Allocation,
    pub cell_ids: Allocation,
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
        let indirect_cells = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: cell_ids.len() as u32,
        });

        let indirect_cells = Allocation::new(device, "indirect_cells", &[indirect_cells]);
        let cell_ids = Allocation::new(device, "cell_ids", cell_ids);

        Self {
            indirect_cells,
            cell_ids,
        }
    }
}

pub struct Output {
    pub indirect_blocks: Allocation,
    pub block_ids: Allocation,
    pub block_table: Allocation,
}

impl PipelinePart for BuildBlocks {
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
        let build_hash_table_from_cells = BuildHashTableFromCells::new(
            context,
            build_hash_table_from_cells::Settings { workgroup_size },
        );
        let allocate_blocks = AllocateBlocks::new(
            context,
            allocate_blocks::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );
        let offsets_to_indirect = OffsetsToIndirect::new(
            context,
            offsets_to_indirect::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );

        let_compiled_module!(
            build_blocks,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (Vector4::<i32>::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (Vector4::<i32>::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64)],
            }
        );

        let_compiled_module!(
            build_hash_table_from_blocks,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (Vector4::<i32>::MIN_BINDING_SIZE, false),
                    (AtomicU32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64)],
            }
        );

        Self {
            build_hash_table_from_cells,
            allocate_blocks,
            build_blocks,
            offsets_to_indirect,
            build_hash_table_from_blocks,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_cells,
            cell_ids,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let build_hash_table_from_cells::Output { block_table, owns } =
            self.build_hash_table_from_cells.record(
                context,
                encoder,
                build_hash_table_from_cells::Input {
                    indirect: indirect_cells.clone(),
                    cell_ids: cell_ids.clone(),
                },
                build_hash_table_from_cells::Parameters,
            )?;

        let allocate_blocks::Output { block_offsets } = self.allocate_blocks.record(
            context,
            encoder,
            allocate_blocks::Input {
                indirect: indirect_cells.clone(),
                owns: owns.clone(),
            },
            allocate_blocks::Parameters,
        )?;

        let offsets_to_indirect::Output {
            new_indirect: indirect_blocks,
            ..
        } = self.offsets_to_indirect.record(
            context,
            encoder,
            offsets_to_indirect::Input {
                indirect: indirect_cells.clone(),
                offsets: block_offsets.clone(),
            },
            offsets_to_indirect::Parameters,
        )?;

        // TODO: this is overkill
        let block_ids = context.allocator()?.allocate::<Vector4<i32>>(
            "block_ids",
            (block_offsets.len::<u32>().get() * 8).try_into().unwrap(),
        )?;

        let mut compute_pass = encoder.begin_compute_pass(self.build_blocks.label);
        compute_pass.set_pipeline(&self.build_blocks.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.build_blocks,
                [
                    indirect_cells.binding(),
                    cell_ids.binding(),
                    owns.binding(),
                    block_offsets.binding(),
                    block_ids.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(indirect_cells.buffer(), indirect_cells.offset());

        drop(indirect_cells);
        drop(cell_ids);
        drop(owns);
        drop(block_offsets);

        drop(compute_pass);

        encoder.clear_buffer(
            block_table.buffer(),
            block_table.offset(),
            Some(block_table.size().get()),
        );

        let mut compute_pass = encoder.begin_compute_pass(self.build_hash_table_from_blocks.label);
        compute_pass.set_pipeline(&self.build_hash_table_from_blocks.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.build_hash_table_from_blocks,
                [
                    indirect_blocks.binding(),
                    block_ids.binding(),
                    block_table.binding(),
                ],
            ),
            &[],
        );
        compute_pass
            .dispatch_workgroups_indirect(indirect_blocks.buffer(), indirect_blocks.offset());

        Ok(Output {
            indirect_blocks,
            block_ids,
            block_table,
        })
    }
}
