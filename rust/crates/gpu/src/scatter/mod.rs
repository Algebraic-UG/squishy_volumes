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

use nalgebra::{Matrix4, Vector4};

use super::*;

pub struct Scatter {
    scatter: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub grid_node_size: f32,
}

pub struct Parameters;

pub struct Input {
    pub indirect_nodes: Allocation,

    pub contributor_offsets: Allocation,
    pub contributors: Allocation,

    pub node_ids_and_collider_bits: Allocation,
    pub particle_tmp: Allocation,
}

#[derive(Clone)]
pub struct InputData<'a> {
    pub contributor_offsets: &'a [u32],
    pub contributors: &'a [u32],
    pub node_ids_and_collider_bits: &'a [NodeIdAndColliderBits],
    pub particle_tmp: &'a [Matrix4<f32>],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings { workgroup_size, .. }: Settings,
        dispatch_limit: NonZeroU32,
        InputData {
            contributor_offsets,
            contributors,
            node_ids_and_collider_bits,
            particle_tmp,
        }: InputData,
    ) -> Result<Self, GpuError> {
        check_length!(contributor_offsets, node_ids_and_collider_bits)?;
        check_length_multiple!(contributors, particle_tmp, 27)?;
        check_indices_valid!(contributor_offsets, contributors)?;
        check_indices_valid!(contributors, particle_tmp)?;

        let indirect_nodes = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: contributor_offsets.len() as u32,
        });

        let indirect_nodes = Allocation::new(device, "indirect_nodes", &[indirect_nodes])?;
        let contributor_offsets =
            Allocation::new(device, "contributor_offsets", contributor_offsets)?;
        let contributors = Allocation::new(device, "contributors", contributors)?;
        let node_ids_and_collider_bits = Allocation::new(
            device,
            "node_ids_and_collider_bits",
            node_ids_and_collider_bits,
        )?;
        let particle_tmp = Allocation::new(device, "particle_tmp", particle_tmp)?;

        Ok(Self {
            indirect_nodes,
            contributor_offsets,
            contributors,
            node_ids_and_collider_bits,
            particle_tmp,
        })
    }
}

pub struct Output {
    pub node_momentums: Allocation,
}

impl PipelinePart for Scatter {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings {
            workgroup_size,
            grid_node_size,
        }: Settings,
    ) -> Self {
        let_compiled_module!(
            scatter,
            CompiledModuleSettings {
                context,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),               // indirect
                    (u32::MIN_BINDING_SIZE, false),                   // contributor_offsets
                    (u32::MIN_BINDING_SIZE, false),                   // contributors
                    (NodeIdAndColliderBits::MIN_BINDING_SIZE, false), // node_ids_and_collider_bits
                    (Matrix4::<f32>::MIN_BINDING_SIZE, false),        // particle_tmp
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),        // node_momentums
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("GRID_NODE_SIZE", grid_node_size as f64),
                ]
            }
        );

        Self { scatter }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_nodes,
            contributor_offsets,
            contributors,
            node_ids_and_collider_bits,
            particle_tmp,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        assert_eq!(indirect_nodes.len::<Indirect>().get(), 1);
        assert_eq!(
            contributors.len::<u32>().get(),
            particle_tmp.len::<Matrix4<f32>>().get() * 27
        );
        assert_eq!(
            contributor_offsets.len::<u32>(),
            node_ids_and_collider_bits.len::<NodeIdAndColliderBits>()
        );
        let node_momentums = context
            .allocator()?
            .allocate::<Vector4<f32>>("node_momentums", contributor_offsets.len::<u32>())?;

        context
            .enter_module(
                encoder,
                &self.scatter,
                [
                    indirect_nodes.binding(),
                    contributor_offsets.binding(),
                    contributors.binding(),
                    node_ids_and_collider_bits.binding(),
                    particle_tmp.binding(),
                    node_momentums.binding(),
                ],
            )
            .dispatch_workgroups_indirect(indirect_nodes.buffer(), indirect_nodes.offset());

        Ok(Output { node_momentums })
    }
}
