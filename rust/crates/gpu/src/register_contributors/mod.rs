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

pub struct RegisterContributors {
    count_contributors: CompiledModule,
    prefix_sum: PrefixSum,
    register_contributors: CompiledModule,

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
    pub indirect_nodes: Allocation,
    pub particle_positions_and_collider_bits: Allocation,
    pub hash_table: Allocation,
    pub node_ids_and_collider_bits: Allocation,
}

pub struct Output {
    pub contributor_offsets: Allocation,
    pub contributors: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
            ..
        }: Settings,
        node_ids_and_collider_bits: &[NodeIdAndColliderBits],
        particle_positions_and_collider_bits: &[PositionAndColliderBits],
    ) -> Result<Self, GpuAllocatorError> {
        let hash_table = hash_table_on_cpu(
            node_ids_and_collider_bits,
            particle_positions_and_collider_bits,
        );

        let indirect_nodes = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: node_ids_and_collider_bits.len() as u32,
        });

        let indirect_nodes = Allocation::new(device, "indirect_nodes", &[indirect_nodes])?;
        let particle_positions_and_collider_bits = Allocation::new(
            device,
            "particle_positions_and_collider_bits",
            particle_positions_and_collider_bits,
        )?;
        let hash_table = Allocation::new(device, "hash_table", &hash_table)?;
        let node_ids_and_collider_bits = Allocation::new(
            device,
            "node_ids_and_collider_bits",
            node_ids_and_collider_bits,
        )?;

        Ok(Self {
            indirect_nodes,
            particle_positions_and_collider_bits,
            hash_table,
            node_ids_and_collider_bits,
        })
    }
}

impl PipelinePart for RegisterContributors {
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
        }: Settings,
    ) -> Self {
        let_compiled_module!(
            count_contributors,
            CompiledModuleSettings {
                context,
                bind_group_entries: [
                    (PositionAndColliderBits::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (NodeIdAndColliderBits::MIN_BINDING_SIZE, false),
                    (AtomicU32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("GRID_NODE_SIZE", grid_node_size as f64),
                ]
            }
        );

        let prefix_sum = PrefixSum::new(
            context,
            prefix_sum::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );

        let_compiled_module!(
            register_contributors,
            CompiledModuleSettings {
                context,
                bind_group_entries: [
                    (PositionAndColliderBits::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (NodeIdAndColliderBits::MIN_BINDING_SIZE, false),
                    (AtomicU32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("GRID_NODE_SIZE", grid_node_size as f64),
                ]
            }
        );

        Self {
            count_contributors,
            prefix_sum,
            register_contributors,
            workgroup_size,
            dispatch_limit,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_nodes,
            particle_positions_and_collider_bits,
            hash_table,
            node_ids_and_collider_bits,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let num_particles = particle_positions_and_collider_bits.len::<PositionAndColliderBits>();
        let num_grid_nodes = node_ids_and_collider_bits.len::<NodeIdAndColliderBits>();
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: num_particles.get() as u32,
        })
        .direct();

        let contributor_counts = context
            .allocator()?
            .allocate::<AtomicU32>("contributor_counts", num_grid_nodes)?;

        encoder
            .scope(Some("clear_contributor_counts"))
            .clear_buffer(
                contributor_counts.buffer(),
                contributor_counts.offset(),
                Some(contributor_counts.size().get()),
            );

        context
            .enter_module(
                encoder,
                &self.count_contributors,
                [
                    particle_positions_and_collider_bits.binding(),
                    hash_table.binding(),
                    node_ids_and_collider_bits.binding(),
                    contributor_counts.binding(),
                ],
            )
            .dispatch_workgroups(x, y, z);

        let prefix_sum::Output {
            prefix_sums: contributor_offsets,
            total_sum,
        } = self.prefix_sum.record(
            context,
            encoder,
            prefix_sum::Input {
                indirect: indirect_nodes,
                numbers: contributor_counts.clone(),
            },
            prefix_sum::Parameters { total_sum: false },
        )?;
        assert!(total_sum.is_none());

        let contributors = context.allocator()?.allocate::<u32>(
            "contributors",
            (num_particles.get() * 27).try_into().unwrap(),
        )?;

        encoder
            .scope(Some("clear_contributor_counts_again"))
            .clear_buffer(
                contributor_counts.buffer(),
                contributor_counts.offset(),
                Some(contributor_counts.size().get()),
            );

        context
            .enter_module(
                encoder,
                &self.register_contributors,
                [
                    particle_positions_and_collider_bits.binding(),
                    hash_table.binding(),
                    node_ids_and_collider_bits.binding(),
                    contributor_counts.binding(),
                    contributor_offsets.binding(),
                    contributors.binding(),
                ],
            )
            .dispatch_workgroups(x, y, z);

        Ok(Output {
            contributor_offsets,
            contributors,
        })
    }
}
