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
    ) -> Self {
        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: node_ids_and_collider_bits.len() as u32,
        });

        let node_ids_and_collider_bits = Allocation::new(
            device,
            "node_ids_and_collider_bits",
            &node_ids_and_collider_bits,
        );
        let indirect = Allocation::new(device, "indirect", &[indirect]);

        Self {
            indirect,
            node_ids_and_collider_bits,
        }
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

    fn new(context: &GpuContext, Settings { workgroup_size }: Settings) -> Self {
        let device = context.device();

        let_compiled_module!(
            node_ids_to_murmur,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (NodeIdAndColliderBits::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64)]
            }
        );

        Self { node_ids_to_murmur }
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

        let mut compute_pass = encoder.begin_compute_pass(self.node_ids_to_murmur.label);
        compute_pass.set_pipeline(&self.node_ids_to_murmur.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.node_ids_to_murmur,
                [
                    indirect.binding(),
                    node_ids_and_collider_bits.binding(),
                    hashes_node_ids.binding(),
                    hashes_node_ids_and_collider_bits.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());
        Ok(Output {
            hashes_node_ids,
            hashes_node_ids_and_collider_bits,
        })
    }
}
