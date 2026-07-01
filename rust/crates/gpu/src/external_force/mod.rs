// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use nalgebra::Vector4;
use std::num::NonZeroU32;

use super::*;

pub struct ExternalForce {
    external_force: CompiledModule,

    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub time_step: f32,
}

pub struct Parameters;

pub struct Input {
    pub gravity: Allocation,
    pub particle_velocities: Allocation,
}

pub struct InputData<'a> {
    pub gravity: Vector4<f32>,
    pub particle_velocities: &'a [Vector4<f32>],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        InputData {
            gravity,
            particle_velocities,
        }: InputData,
    ) -> Result<Self, GpuError> {
        let gravity = Allocation::new(device, "gravity", &[gravity])?;
        let particle_velocities =
            Allocation::new(device, "particle_velocities", particle_velocities)?;

        Ok(Self {
            gravity,
            particle_velocities,
        })
    }
}

pub struct Output;

impl PipelinePart for ExternalForce {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            time_step,
        }: Settings,
    ) -> Self {
        let_compiled_module!(
            external_force,
            CompiledModuleSettings {
                context,
                bind_group_entries: [
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // gravity
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // particle_velocities
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("TIME_STEP", time_step as f64),
                ]
            }
        );

        Self {
            external_force,
            workgroup_size,
            dispatch_limit,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            gravity,
            particle_velocities,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: particle_velocities.len::<Vector4<f32>>().get() as u32,
        })
        .direct();

        context
            .enter_module(
                encoder,
                &self.external_force,
                [gravity.binding(), particle_velocities.binding()],
            )
            .dispatch_workgroups(x, y, z);

        Ok(Output)
    }
}
