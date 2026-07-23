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

use nalgebra::{Matrix4x3, Vector4};

use super::*;

pub struct Collect {
    collect: CompiledModule,

    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub grid_node_size: f32,
    pub time_step: f32,
    pub table_tries: u32,
}

pub struct Parameters;

pub struct Input {
    pub hash_table: Allocation,

    pub node_ids_and_collider_bits: Allocation,
    pub node_momentums: Allocation,

    pub particle_positions_and_collider_bits: Allocation,
    pub particle_position_gradients: Allocation,
    pub particle_velocities: Allocation,
    pub particle_velocity_gradients: Allocation,
}

#[derive(Clone)]
pub struct InputData<'a> {
    pub node_ids_and_collider_bits: &'a [NodeIdAndColliderBits],
    pub node_momentums: &'a [Vector4<f32>],

    pub particle_positions_and_collider_bits: &'a [PositionAndColliderBits],
    pub particle_position_gradients: &'a [Matrix4x3<f32>],
    pub particle_velocities: &'a [Vector4<f32>],
    pub particle_velocity_gradients: &'a [Matrix4x3<f32>],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        InputData {
            node_ids_and_collider_bits,
            node_momentums,
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        }: InputData,
    ) -> Result<Self, GpuError> {
        check_length!(node_ids_and_collider_bits, node_momentums)?;
        check_length!(
            particle_positions_and_collider_bits,
            particle_position_gradients
        )?;
        check_length!(particle_positions_and_collider_bits, particle_velocities)?;
        check_length!(
            particle_positions_and_collider_bits,
            particle_velocity_gradients
        )?;

        let hash_table = build_hash_table_on_cpu(node_ids_and_collider_bits);

        let hash_table = Allocation::new(device, "hash_table", &hash_table)?;
        let node_ids_and_collider_bits = Allocation::new(
            device,
            "node_ids_and_collider_bits",
            node_ids_and_collider_bits,
        )?;
        let node_momentums = Allocation::new(device, "node_momentums", node_momentums)?;
        let particle_positions_and_collider_bits = Allocation::new(
            device,
            "particle_positions_and_collider_bits",
            particle_positions_and_collider_bits,
        )?;
        let particle_position_gradients = Allocation::new(
            device,
            "particle_position_gradients",
            particle_position_gradients,
        )?;
        let particle_velocities =
            Allocation::new(device, "particle_velocities", particle_velocities)?;
        let particle_velocity_gradients = Allocation::new(
            device,
            "particle_velocity_gradients",
            particle_velocity_gradients,
        )?;

        Ok(Self {
            hash_table,
            node_ids_and_collider_bits,
            node_momentums,
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        })
    }
}

pub struct Output;

// this is actually still the input, but modified.. w/e
pub struct OutputData {
    pub particle_positions_and_collider_bits: Vec<PositionAndColliderBits>,
    pub particle_position_gradients: Vec<Matrix4x3<f32>>,
    pub particle_velocities: Vec<Vector4<f32>>,
    pub particle_velocity_gradients: Vec<Matrix4x3<f32>>,
}

impl PipelinePart for Collect {
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
            time_step,
            table_tries,
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            collect,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (u32::MIN_BINDING_SIZE, false),                     // hash_table
                    (NodeIdAndColliderBits::MIN_BINDING_SIZE, false), // node_ids_and_collider_bits
                    (Vector4::<i32>::MIN_BINDING_SIZE, false),        // node_momentums
                    (PositionAndColliderBits::MIN_BINDING_SIZE, false), // particle_positions_and_collider_bits
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false), // particle_position_gradients
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),   // particle_velocities
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false), // particle_velocity_gradients
                ],
                immediate_size: 0,
                constants: [
                    ("GRID_NODE_SIZE", grid_node_size as f64),
                    ("TIME_STEP", time_step as f64),
                    ("TABLE_TRIES", table_tries as f64),
                ]
            }
        );

        Ok(Self {
            collect,
            workgroup_size,
            dispatch_limit,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            hash_table,
            node_ids_and_collider_bits,
            node_momentums,
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: particle_positions_and_collider_bits
                .len::<PositionAndColliderBits>()
                .get() as u32,
        })
        .direct();

        context
            .enter_module(
                encoder,
                &self.collect,
                [
                    hash_table.binding(),
                    node_ids_and_collider_bits.binding(),
                    node_momentums.binding(),
                    particle_positions_and_collider_bits.binding(),
                    particle_position_gradients.binding(),
                    particle_velocities.binding(),
                    particle_velocity_gradients.binding(),
                ],
            )
            .dispatch_workgroups(x, y, z);

        Ok(Output)
    }
}
