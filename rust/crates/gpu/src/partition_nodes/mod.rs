// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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
    ) -> Self {
        let_compiled_module!(
            partition_nodes,
            CompiledModuleSettings {
                context,
                bind_group_entries: [
                    (PositionAndColliderBits::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (AtomicU32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("GRID_NODE_SIZE", grid_node_size as f64),
                    ("TABLE_TRIES", table_tries as f64),
                ]
            }
        );

        Self {
            partition_nodes,

            workgroup_size,
            dispatch_limit,
        }
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
