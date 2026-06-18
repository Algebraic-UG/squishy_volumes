// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU32;

use nalgebra::Vector4;

#[cfg(test)]
mod test;

use super::*;

pub struct RegisterContributors {
    count_contributors: CompiledModule,
    prefix_sum: PrefixSum,
    register_contributors: CompiledModule,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub cell_size: f32,
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
            cell_size,
            ..
        }: Settings,
        positions: &[Vector4<f32>],
    ) -> Self {
        Self {
            indirect_nodes: todo!(),
            particle_positions_and_collider_bits: todo!(),
            hash_table: todo!(),
            node_ids_and_collider_bits: todo!(),
        }
    }
}

impl PipelinePart for RegisterContributors {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            cell_size,
        }: Settings,
    ) -> Self {
        Self {
            count_contributors: todo!(),
            prefix_sum: todo!(),
            register_contributors: todo!(),
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
        Ok(Output {
            contributor_offsets: todo!(),
            contributors: todo!(),
        })
    }
}
