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

use std::{num::NonZeroU32, sync::atomic::AtomicU32};

#[cfg(test)]
mod test;

use super::*;

pub struct PartitionNodes {
    partition_nodes: CompiledModule,

    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub grid_node_size: f32,
    pub table_tries: u32,
}

pub struct Parameters {
    pub max_num_grid_nodes: NonZeroU32,
}

pub struct Input {
    pub particle_positions_and_collider_bits: Allocation,
}

pub struct Output {
    pub owns: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        particle_positions_and_collider_bits: &[PositionAndColliderBits],
    ) -> Result<Self, GpuAllocatorError> {
        let particle_positions_and_collider_bits = Allocation::new(
            device,
            "particle_positions_and_collider_bits",
            particle_positions_and_collider_bits,
        )?;

        Ok(Self {
            particle_positions_and_collider_bits,
        })
    }
}

impl PipelinePart for PartitionNodes {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            grid_node_size,
            table_tries,
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            partition_nodes,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (PositionAndColliderBits::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (AtomicU32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("GRID_NODE_SIZE", grid_node_size as f64),
                    ("TABLE_TRIES", table_tries as f64),
                ]
            }
        );

        Ok(Self {
            partition_nodes,

            workgroup_size,
            dispatch_limit,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            particle_positions_and_collider_bits,
        }: Input,
        Parameters { max_num_grid_nodes }: Parameters,
    ) -> Result<Output, GpuError> {
        let num_particles = particle_positions_and_collider_bits.len::<PositionAndColliderBits>();
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: num_particles.get() as u32,
        })
        .direct();

        let owns = context
            .allocator()?
            .allocate::<u32>("owns", num_particles)?;
        let hash_table = context.allocator()?.allocate::<AtomicU32>(
            "hash_table",
            (max_num_grid_nodes.get() as u64 * 2)
                .next_power_of_two()
                .try_into()
                .unwrap(),
        )?;

        encoder
            .scope(Some("clear_hash_table_particles"))
            .clear_buffer(
                hash_table.buffer(),
                hash_table.offset(),
                Some(hash_table.size().get()),
            );

        context
            .enter_module(
                encoder,
                &self.partition_nodes,
                [
                    particle_positions_and_collider_bits.binding(),
                    owns.binding(),
                    hash_table.binding(),
                ],
            )
            .dispatch_workgroups(x, y, z);

        Ok(Output { owns })
    }
}
