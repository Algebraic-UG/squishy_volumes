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

use nalgebra::Vector4;

use super::*;

pub struct MeldGrid {
    meld_grid: CompiledModule,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect_nodes: Allocation,
    pub hash_table: Allocation,
    pub node_ids_and_collider_bits: Allocation,
    pub node_momentums_in: Allocation,
}

#[derive(Clone)]
pub struct InputData<'a> {
    pub node_ids_and_collider_bits: &'a [NodeIdAndColliderBits],
    pub node_momentums_in: &'a [Vector4<f32>],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings { workgroup_size }: Settings,
        dispatch_limit: NonZeroU32,
        InputData {
            node_ids_and_collider_bits,
            node_momentums_in,
        }: InputData,
    ) -> Self {
        assert_eq!(node_ids_and_collider_bits.len(), node_momentums_in.len());

        let indirect_nodes = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: node_ids_and_collider_bits.len() as u32,
        });

        let indirect_nodes = Allocation::new(device, "indirect_nodes", &[indirect_nodes]);
        let hash_table = build_hash_table_on_cpu(node_ids_and_collider_bits);
        let hash_table = Allocation::new(device, "hash_table", &hash_table);
        let node_ids_and_collider_bits = Allocation::new(
            device,
            "node_ids_and_collider_bits",
            node_ids_and_collider_bits,
        );
        let node_momentums_in = Allocation::new(device, "node_momentums_in", node_momentums_in);

        Self {
            indirect_nodes,
            hash_table,
            node_ids_and_collider_bits,
            node_momentums_in,
        }
    }
}

pub struct Output {
    pub node_momentums_out: Allocation,
}

impl PipelinePart for MeldGrid {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &GpuContext, Settings { workgroup_size }: Settings) -> Self {
        let_compiled_module!(
            meld_grid,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                    (NodeIdAndColliderBits::MIN_BINDING_SIZE, false),
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64)]
            }
        );
        Self { meld_grid }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_nodes,
            hash_table,
            node_ids_and_collider_bits,
            node_momentums_in,
        }: Input,
        Parameters {}: Parameters,
    ) -> Result<Output, GpuError> {
        assert_eq!(
            node_ids_and_collider_bits.len::<NodeIdAndColliderBits>(),
            node_momentums_in.len::<Vector4<f32>>()
        );

        let node_momentums_out = context.allocator()?.allocate::<Vector4<f32>>(
            "node_momentums_out",
            node_momentums_in.len::<Vector4<f32>>(),
        )?;

        let mut compute_pass = encoder.begin_compute_pass(self.meld_grid.label);
        compute_pass.set_pipeline(&self.meld_grid.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.meld_grid,
                [
                    indirect_nodes.binding(),
                    hash_table.binding(),
                    node_ids_and_collider_bits.binding(),
                    node_momentums_in.binding(),
                    node_momentums_out.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(indirect_nodes.buffer(), indirect_nodes.offset());

        Ok(Output { node_momentums_out })
    }
}
