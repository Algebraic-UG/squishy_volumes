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

pub struct PrepareGrid {
    partition_nodes: PartitionNodes,
    bits_to_pops: BitsToPops,
    prefix_sum: PrefixSum,
    len_to_indirect: LenToIndirect,
    build_nodes: CompiledModule,
    build_hash_tables: BuildHashTables,
    fill_multi_map: CompiledModule,

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
    pub indirect_particles: Allocation,
    pub particle_positions_and_collider_bits: Allocation,
}

pub struct Output {
    pub indirect_nodes: Allocation,
    pub hash_table: Allocation,
    pub node_ids_and_collider_bits: Allocation,
    pub hash_table_multi: Allocation,
    pub multi_offsets: Allocation,
    pub multi: Allocation,
}

pub struct OutputData {
    pub indirect_nodes: Indirect,
    pub hash_table: Vec<u32>,
    pub node_ids_and_collider_bits: Vec<NodeIdAndColliderBits>,
    pub hash_table_multi: Vec<u32>,
    pub multi_offsets: Vec<u32>,
    pub multi: Vec<u32>,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
            ..
        }: Settings,
        particle_positions_and_collider_bits: &[PositionAndColliderBits],
    ) -> Result<Self, GpuAllocatorError> {
        let indirect_particles = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: particle_positions_and_collider_bits.len() as u32,
        });
        let indirect_particles =
            Allocation::new(device, "indirect_particles", &[indirect_particles])?;
        let particle_positions_and_collider_bits = Allocation::new(
            device,
            "particle_positions_and_collider_bits",
            particle_positions_and_collider_bits,
        )?;

        Ok(Self {
            indirect_particles,
            particle_positions_and_collider_bits,
        })
    }
}

impl PipelinePart for PrepareGrid {
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
        let partition_nodes = PartitionNodes::new(
            context,
            partition_nodes::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size,
                table_tries,
            },
        )?;

        let bits_to_pops = BitsToPops::new(context, bits_to_pops::Settings { workgroup_size })?;
        let prefix_sum = PrefixSum::new(
            context,
            prefix_sum::Settings {
                workgroup_size,
                dispatch_limit,
            },
        )?;
        let len_to_indirect = LenToIndirect::new(
            context,
            len_to_indirect::Settings {
                workgroup_size,
                dispatch_limit,
            },
        )?;
        let_compiled_module!(
            build_nodes,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (PositionAndColliderBits::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (NodeIdAndColliderBits::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("GRID_NODE_SIZE", grid_node_size as f64),]
            }
        );

        let build_hash_tables = BuildHashTables::new(
            context,
            build_hash_tables::Settings {
                workgroup_size,
                table_tries,
            },
        )?;

        let_compiled_module!(
            fill_multi_map,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (NodeIdAndColliderBits::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (AtomicU32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("TABLE_TRIES", table_tries as f64),]
            }
        );

        Ok(Self {
            partition_nodes,
            bits_to_pops,
            prefix_sum,
            len_to_indirect,
            build_nodes,
            build_hash_tables,
            fill_multi_map,

            workgroup_size,
            dispatch_limit,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_particles,
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

        let partition_nodes::Output { owns } = self.partition_nodes.record(
            context,
            encoder,
            partition_nodes::Input {
                particle_positions_and_collider_bits: particle_positions_and_collider_bits.clone(),
            },
            partition_nodes::Parameters { max_num_grid_nodes },
        )?;

        let bits_to_pops::Output { pops } = self.bits_to_pops.record(
            context,
            encoder,
            bits_to_pops::Input {
                indirect: indirect_particles.clone(),
                bits: owns.clone(),
            },
            bits_to_pops::Parameters,
        )?;

        let prefix_sum::Output {
            prefix_sums: offsets,
            total_sum: Some(total_nodes),
        } = self.prefix_sum.record(
            context,
            encoder,
            prefix_sum::Input {
                indirect: indirect_particles,
                numbers: pops,
            },
            prefix_sum::Parameters { total_sum: true },
        )?
        else {
            unreachable!("we asked for the total sum");
        };

        let len_to_indirect::Output {
            new_indirect: indirect_nodes,
        } = self.len_to_indirect.record(
            context,
            encoder,
            len_to_indirect::Input { len: total_nodes },
            len_to_indirect::Parameters {
                limit: max_num_grid_nodes.get(),
            },
        )?;

        let node_ids_and_collider_bits = context.allocator()?.allocate::<NodeIdAndColliderBits>(
            "node_ids_and_collider_bits",
            max_num_grid_nodes.into(),
        )?;

        context
            .enter_module(
                encoder,
                &self.build_nodes,
                [
                    particle_positions_and_collider_bits.binding(),
                    owns.binding(),
                    offsets.binding(),
                    node_ids_and_collider_bits.binding(),
                ],
            )
            .dispatch_workgroups(x, y, z);

        drop(owns);
        drop(offsets);

        let build_hash_tables::Output {
            hash_table,
            hash_table_multi,
            multi_counts,
        } = self.build_hash_tables.record(
            context,
            encoder,
            build_hash_tables::Input {
                indirect_nodes: indirect_nodes.clone(),
                node_ids_and_collider_bits: node_ids_and_collider_bits.clone(),
            },
            build_hash_tables::Parameters,
        )?;

        let prefix_sum::Output {
            prefix_sums: multi_offsets,
            total_sum: None,
        } = self.prefix_sum.record(
            context,
            encoder,
            prefix_sum::Input {
                indirect: indirect_nodes.clone(),
                numbers: multi_counts.clone(),
            },
            prefix_sum::Parameters { total_sum: false },
        )?
        else {
            unreachable!("we didn't ask for the total sum");
        };

        encoder
            .scope(Some("clear_multi_counts_again"))
            .clear_buffer(
                multi_counts.buffer(),
                multi_counts.offset(),
                Some(multi_counts.size().get()),
            );

        let multi = context
            .allocator()?
            .allocate::<u32>("multi", max_num_grid_nodes.into())?;

        context
            .enter_module(
                encoder,
                &self.fill_multi_map,
                [
                    indirect_nodes.binding(),
                    node_ids_and_collider_bits.binding(),
                    hash_table_multi.binding(),
                    multi_counts.binding(),
                    multi_offsets.binding(),
                    multi.binding(),
                ],
            )
            .dispatch_workgroups_indirect(indirect_nodes.buffer(), indirect_nodes.offset());

        Ok(Output {
            indirect_nodes,
            hash_table,
            node_ids_and_collider_bits,
            hash_table_multi,
            multi_offsets,
            multi,
        })
    }
}
