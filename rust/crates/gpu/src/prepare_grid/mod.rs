// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    num::{NonZeroU32, NonZeroU64},
    sync::atomic::AtomicU32,
};

#[cfg(test)]
mod test;

use super::*;

pub struct PrepareGrid {
    partition_nodes: CompiledModule,
    bits_to_pops: BitsToPops,
    prefix_sum: PrefixSum,
    len_to_indirect: LenToIndirect,
    build_nodes: CompiledModule,
    build_hash_table: CompiledModule,

    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub grid_node_size: f32,
}

pub struct Parameters;

pub struct Input {
    pub indirect_particles: Allocation,
    pub particle_positions_and_collider_bits: Allocation,
}

pub struct Output {
    pub indirect_nodes: Allocation,
    pub hash_table: Allocation,
    pub node_ids_and_collider_bits: Allocation,
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
    ) -> Self {
        let indirect_particles = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: particle_positions_and_collider_bits.len() as u32,
        });
        let indirect_particles =
            Allocation::new(device, "indirect_particles", &[indirect_particles]);
        let particle_positions_and_collider_bits = Allocation::new(
            device,
            "particle_positions_and_collider_bits",
            particle_positions_and_collider_bits,
        );

        Self {
            indirect_particles,
            particle_positions_and_collider_bits,
        }
    }
}

impl PipelinePart for PrepareGrid {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            grid_node_size,
        }: Settings,
    ) -> Self {
        let_compiled_module!(
            partition_nodes,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (PositionAndColliderBits::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (AtomicU32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("GRID_NODE_SIZE", grid_node_size as f64),
                ]
            }
        );

        let bits_to_pops = BitsToPops::new(context, bits_to_pops::Settings { workgroup_size });
        let prefix_sum = PrefixSum::new(
            context,
            prefix_sum::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );
        let len_to_indirect = LenToIndirect::new(
            context,
            len_to_indirect::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );
        let_compiled_module!(
            build_nodes,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (PositionAndColliderBits::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (NodeIdAndColliderBits::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("GRID_NODE_SIZE", grid_node_size as f64),
                ]
            }
        );
        let_compiled_module!(
            build_hash_table,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (NodeIdAndColliderBits::MIN_BINDING_SIZE, false),
                    (AtomicU32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64),]
            }
        );

        Self {
            partition_nodes,
            bits_to_pops,
            prefix_sum,
            len_to_indirect,
            build_nodes,
            build_hash_table,

            workgroup_size,
            dispatch_limit,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_particles,
            particle_positions_and_collider_bits,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let num_particles = particle_positions_and_collider_bits.len::<PositionAndColliderBits>();
        let max_num_nodes: NonZeroU64 = (num_particles.get() * 27).try_into().unwrap();
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: num_particles.get() as u32,
        })
        .direct();

        let owns = context
            .allocator()?
            .allocate::<u32>("owns", num_particles)?;
        let hash_table = context
            .allocator()?
            .allocate::<AtomicU32>("hash_table", max_num_nodes)?;

        encoder.clear_buffer(
            hash_table.buffer(),
            hash_table.offset(),
            Some(hash_table.size().get()),
        );

        let mut compute_pass = encoder.begin_compute_pass(self.partition_nodes.label);
        compute_pass.set_pipeline(&self.partition_nodes.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.partition_nodes,
                [
                    particle_positions_and_collider_bits.binding(),
                    owns.binding(),
                    hash_table.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups(x, y, z);
        drop(compute_pass);

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
            unreachable!("we asked prefix sum for the total sum");
        };

        let len_to_indirect::Output {
            new_indirect: indirect_nodes,
        } = self.len_to_indirect.record(
            context,
            encoder,
            len_to_indirect::Input { len: total_nodes },
            len_to_indirect::Parameters,
        )?;

        let node_ids_and_collider_bits = context
            .allocator()?
            .allocate::<NodeIdAndColliderBits>("node_ids_and_collider_bits", max_num_nodes)?;

        let mut compute_pass = encoder.begin_compute_pass(self.build_nodes.label);
        compute_pass.set_pipeline(&self.build_nodes.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.build_nodes,
                [
                    particle_positions_and_collider_bits.binding(),
                    owns.binding(),
                    offsets.binding(),
                    node_ids_and_collider_bits.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups(x, y, z);
        drop(compute_pass);

        drop(owns);
        drop(offsets);

        encoder.clear_buffer(
            hash_table.buffer(),
            hash_table.offset(),
            Some(hash_table.size().get()),
        );

        let mut compute_pass = encoder.begin_compute_pass(self.build_hash_table.label);
        compute_pass.set_pipeline(&self.build_hash_table.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.build_hash_table,
                [
                    indirect_nodes.binding(),
                    node_ids_and_collider_bits.binding(),
                    hash_table.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(indirect_nodes.buffer(), indirect_nodes.offset());

        Ok(Output {
            indirect_nodes,
            hash_table,
            node_ids_and_collider_bits,
        })
    }
}
