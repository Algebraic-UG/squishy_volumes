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

#[cfg(test)]
mod test;

use std::num::NonZeroU32;

use super::*;

pub struct NodeIdsToMurmur {
    node_ids_to_murmur: CompiledModule,
}

pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub node_ids_and_collider_bits: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        workgroup_size: NonZeroU32,
        dispatch_limit: NonZeroU32,
        node_ids_and_collider_bits: &[NodeIdAndColliderBits],
    ) -> Result<Self, GpuAllocatorError> {
        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: node_ids_and_collider_bits.len() as u32,
        });

        let node_ids_and_collider_bits = Allocation::new(
            device,
            "node_ids_and_collider_bits",
            node_ids_and_collider_bits,
        )?;
        let indirect = Allocation::new(device, "indirect", &[indirect])?;

        Ok(Self {
            indirect,
            node_ids_and_collider_bits,
        })
    }
}

pub struct Output {
    pub hashes_node_ids: Allocation,
    pub hashes_node_ids_and_collider_bits: Allocation,
}

impl PipelinePart for NodeIdsToMurmur {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings { workgroup_size }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            node_ids_to_murmur,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (NodeIdAndColliderBits::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: []
            }
        );

        Ok(Self { node_ids_to_murmur })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            node_ids_and_collider_bits,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let hashes_node_ids = context.allocator()?.allocate::<u32>(
            "hashes_node_ids",
            node_ids_and_collider_bits.len::<NodeIdAndColliderBits>(),
        )?;
        let hashes_node_ids_and_collider_bits = context.allocator()?.allocate::<u32>(
            "hashes_node_ids_and_collider_bits",
            node_ids_and_collider_bits.len::<NodeIdAndColliderBits>(),
        )?;

        context
            .enter_module(
                encoder,
                &self.node_ids_to_murmur,
                [
                    indirect.binding(),
                    node_ids_and_collider_bits.binding(),
                    hashes_node_ids.binding(),
                    hashes_node_ids_and_collider_bits.binding(),
                ],
            )
            .dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());
        Ok(Output {
            hashes_node_ids,
            hashes_node_ids_and_collider_bits,
        })
    }
}
