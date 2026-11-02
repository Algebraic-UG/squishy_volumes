// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    num::{NonZeroU32, NonZeroU64},
    sync::atomic::AtomicU32,
};

#[cfg(test)]
mod test;

use super::*;

pub struct BuildHashTables {
    build_hash_tables: CompiledModule,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub table_tries: u32,
}

pub struct Parameters;

pub struct Input {
    pub indirect_nodes: Allocation,
    pub node_ids_and_collider_bits: Allocation,
}

pub struct Output {
    pub hash_table: Allocation,
    pub hash_table_multi: Allocation,
    pub multi_counts: Allocation,
}

pub struct OutputData {
    pub hash_table: Vec<u32>,
    pub hash_table_multi: Vec<u32>,
    pub multi_counts: Vec<u32>,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        dispatch_limit: NonZeroU32,
        Settings { workgroup_size, .. }: Settings,
        node_ids_and_collider_bits: &[NodeIdAndColliderBits],
    ) -> Result<Self, GpuAllocatorError> {
        let indirect_nodes = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: node_ids_and_collider_bits.len() as u32,
        });

        let indirect_nodes = Allocation::new(device, "indirect_nodes", &[indirect_nodes])?;

        let node_ids_and_collider_bits = Allocation::new(
            device,
            "node_ids_and_collider_bits",
            node_ids_and_collider_bits,
        )?;

        Ok(Self {
            indirect_nodes,
            node_ids_and_collider_bits,
        })
    }
}

impl PipelinePart for BuildHashTables {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings {
            workgroup_size,
            table_tries,
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            build_hash_tables,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (NodeIdAndColliderBits::MIN_BINDING_SIZE, false),
                    (AtomicU32::MIN_BINDING_SIZE, false),
                    (AtomicU32::MIN_BINDING_SIZE, false),
                    (AtomicU32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("TABLE_TRIES", table_tries as f64),]
            }
        );

        Ok(Self { build_hash_tables })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_nodes,
            node_ids_and_collider_bits,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let max_num_nodes: NonZeroU64 = node_ids_and_collider_bits.len::<NodeIdAndColliderBits>();
        let hash_tables_size = (max_num_nodes.get() * 2)
            .next_power_of_two()
            .try_into()
            .unwrap();
        let hash_table = context
            .allocator()?
            .allocate::<AtomicU32>("hash_table", hash_tables_size)?;
        let hash_table_multi = context
            .allocator()?
            .allocate::<AtomicU32>("hash_table_multi", hash_tables_size)?;
        let multi_counts = context
            .allocator()?
            .allocate::<AtomicU32>("multi_counts", max_num_nodes)?;

        encoder.scope(Some("clear_hash_table")).clear_buffer(
            hash_table.buffer(),
            hash_table.offset(),
            Some(hash_table.size().get()),
        );
        encoder.scope(Some("clear_hash_table_multi")).clear_buffer(
            hash_table_multi.buffer(),
            hash_table_multi.offset(),
            Some(hash_table_multi.size().get()),
        );
        encoder.scope(Some("clear_multi_counts")).clear_buffer(
            multi_counts.buffer(),
            multi_counts.offset(),
            Some(multi_counts.size().get()),
        );

        context
            .enter_module(
                encoder,
                &self.build_hash_tables,
                [
                    indirect_nodes.binding(),
                    node_ids_and_collider_bits.binding(),
                    hash_table.binding(),
                    hash_table_multi.binding(),
                    multi_counts.binding(),
                ],
            )
            .dispatch_workgroups_indirect(indirect_nodes.buffer(), indirect_nodes.offset());

        Ok(Output {
            hash_table,
            hash_table_multi,
            multi_counts,
        })
    }
}
