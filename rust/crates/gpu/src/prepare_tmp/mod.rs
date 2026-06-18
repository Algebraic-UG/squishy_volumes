// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU32;

#[cfg(test)]
mod test;

use nalgebra::{Matrix4, Matrix4x3, Vector4};

use super::*;

pub struct PrepareTmp {
    prepare_tmp: CompiledModule,

    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub grid_node_size: f32,
    pub time_step: f32,
}

pub struct Parameters;

pub struct Input {
    pub particle_masses: Allocation,
    pub particle_initial_volumes: Allocation,
    pub particle_parameters: Allocation,
    pub particle_position_gradients: Allocation,
    pub particle_velocities: Allocation,
    pub particle_velocity_gradients: Allocation,
}

#[derive(Clone)]
pub struct InputData<'a> {
    pub particle_masses: &'a [f32],
    pub particle_initial_volumes: &'a [f32],
    pub particle_parameters: &'a [particle_parameters::Device],
    pub particle_position_gradients: &'a [Matrix4x3<f32>],
    pub particle_velocities: &'a [Vector4<f32>],
    pub particle_velocity_gradients: &'a [Matrix4x3<f32>],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        InputData {
            particle_masses,
            particle_initial_volumes,
            particle_parameters,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        }: InputData,
    ) -> Self {
        let particle_masses = Allocation::new(device, "particle_masses", particle_masses);
        let particle_initial_volumes =
            Allocation::new(device, "particle_initial_volumes", particle_initial_volumes);
        let particle_parameters =
            Allocation::new(device, "particle_parameters", particle_parameters);
        let particle_position_gradients = Allocation::new(
            device,
            "particle_position_gradients",
            particle_position_gradients,
        );
        let particle_velocities =
            Allocation::new(device, "particle_velocities", particle_velocities);
        let particle_velocity_gradients = Allocation::new(
            device,
            "particle_velocity_gradients",
            particle_velocity_gradients,
        );

        Self {
            particle_masses,
            particle_initial_volumes,
            particle_parameters,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        }
    }
}

pub struct Output {
    pub particle_tmp: Allocation,
}

impl PipelinePart for PrepareTmp {
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
            time_step,
        }: Settings,
    ) -> Self {
        let_compiled_module!(
            prepare_tmp,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (f32::MIN_BINDING_SIZE, false),
                    (f32::MIN_BINDING_SIZE, false),
                    (particle_parameters::Device::MIN_BINDING_SIZE, false),
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),
                    (Matrix4::<f32>::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("GRID_NODE_SIZE", grid_node_size as f64),
                    ("TIME_STEP", time_step as f64),
                ]
            }
        );

        Self {
            prepare_tmp,
            workgroup_size,
            dispatch_limit,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            particle_masses,
            particle_initial_volumes,
            particle_parameters,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let num_particles = particle_masses.len::<f32>();
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: num_particles.get() as u32,
        })
        .direct();

        let particle_tmp = context
            .allocator()?
            .allocate::<Matrix4<f32>>("particle_tmp", num_particles)?;

        let mut compute_pass = encoder.begin_compute_pass(self.prepare_tmp.label);
        compute_pass.set_pipeline(&self.prepare_tmp.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.prepare_tmp,
                [
                    particle_masses.binding(),
                    particle_initial_volumes.binding(),
                    particle_parameters.binding(),
                    particle_position_gradients.binding(),
                    particle_velocities.binding(),
                    particle_velocity_gradients.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups(x, y, z);
        drop(compute_pass);

        Ok(Output { particle_tmp })
    }
}
