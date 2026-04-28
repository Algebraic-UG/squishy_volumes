// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU32;

use nalgebra::{Matrix4x3, Vector4};

#[cfg(test)]
mod test;

use super::*;

pub struct PermuteParticles {
    permute_particles: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub permutation: Allocation,
    pub indices_in: Allocation,
    pub masses_in: Allocation,
    pub initial_volumes_in: Allocation,
    pub parameters_in: Allocation,
    pub positions_in: Allocation,
    pub position_gradients_in: Allocation,
    pub velocities_in: Allocation,
    pub velocity_gradients_in: Allocation,
}

pub struct InputData<'a> {
    pub permutation: &'a [u32],
    pub indices: &'a [u32],
    pub masses: &'a [f32],
    pub initial_volumes: &'a [f32],
    pub parameters: &'a [particle_parameters::Device],
    pub positions: &'a [Vector4<f32>],
    pub position_gradients: &'a [Matrix4x3<f32>],
    pub velocities: &'a [Vector4<f32>],
    pub velocity_gradients: &'a [Matrix4x3<f32>],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        workgroup_size: NonZeroU32,
        dispatch_limit: NonZeroU32,
        InputData {
            permutation,
            indices,
            masses,
            initial_volumes,
            parameters,
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
        }: InputData,
    ) -> Self {
        assert_eq!(permutation.len(), indices.len());
        assert_eq!(permutation.len(), masses.len());
        assert_eq!(permutation.len(), initial_volumes.len());
        assert_eq!(permutation.len(), parameters.len());
        assert_eq!(permutation.len(), positions.len());
        assert_eq!(permutation.len(), position_gradients.len());
        assert_eq!(permutation.len(), velocities.len());
        assert_eq!(permutation.len(), velocity_gradients.len());
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: permutation.len() as u32,
        });

        let indirect = Allocation::new(device, "indirect", &[indirect]);
        let permutation = Allocation::new(device, "permutation", permutation);
        let indices_in = Allocation::new(device, "indices_in", indices);
        let masses_in = Allocation::new(device, "masses_in", masses);
        let initial_volumes_in = Allocation::new(device, "initial_volumes_in", initial_volumes);
        let parameters_in = Allocation::new(device, "parameters_in", parameters);
        let positions_in = Allocation::new(device, "positions_in", positions);
        let position_gradients_in =
            Allocation::new(device, "position_gradients_in", position_gradients);
        let velocities_in = Allocation::new(device, "velocities_in", velocities);
        let velocity_gradients_in =
            Allocation::new(device, "velocity_gradients_in", velocity_gradients);

        Self {
            indirect,
            permutation,
            indices_in,
            masses_in,
            initial_volumes_in,
            parameters_in,
            positions_in,
            position_gradients_in,
            velocities_in,
            velocity_gradients_in,
        }
    }
}

pub struct Output {
    pub indices_out: Allocation,
    pub masses_out: Allocation,
    pub initial_volumes_out: Allocation,
    pub parameters_out: Allocation,
    pub positions_out: Allocation,
    pub position_gradients_out: Allocation,
    pub velocities_out: Allocation,
    pub velocity_gradients_out: Allocation,
}

impl PipelinePart for PermuteParticles {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &GpuContext, settings: Settings) -> Self {
        let device = context.device();
        use particle_parameters::Device;

        let_compiled_module!(
            permute_particles,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),          // indirect
                    (u32::MIN_BINDING_SIZE, false),              // permutation
                    (u32::MIN_BINDING_SIZE, false),              // indices_in
                    (f32::MIN_BINDING_SIZE, false),              // masses_in
                    (f32::MIN_BINDING_SIZE, false),              // initial_volumes_in
                    (Device::MIN_BINDING_SIZE, false),           // parameters_in
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),   // positions_in
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false), // position_gradients_in
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),   // velocities_in
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false), // velocity_gradients_in
                    (u32::MIN_BINDING_SIZE, false),              // indices_out
                    (f32::MIN_BINDING_SIZE, false),              // masses_out
                    (f32::MIN_BINDING_SIZE, false),              // initial_volumes_out
                    (Device::MIN_BINDING_SIZE, false),           // parameters_out
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),   // positions_out
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false), // position_gradients_out
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),   // velocities_out
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false), // velocity_gradients_out
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", settings.workgroup_size.get() as f64)],
            }
        );

        Self { permute_particles }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            permutation,
            indices_in,
            masses_in,
            initial_volumes_in,
            parameters_in,
            positions_in,
            position_gradients_in,
            velocities_in,
            velocity_gradients_in,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let indices_out = context
            .allocator()?
            .allocate::<u32>("indices_out", indices_in.len::<u32>())?;
        let masses_out = context
            .allocator()?
            .allocate::<f32>("masses_out", masses_in.len::<f32>())?;
        let initial_volumes_out = context
            .allocator()?
            .allocate::<f32>("initial_volumes_out", initial_volumes_in.len::<f32>())?;
        let parameters_out = context
            .allocator()?
            .allocate::<particle_parameters::Device>(
                "parameters_out",
                parameters_in.len::<particle_parameters::Device>(),
            )?;
        let positions_out = context
            .allocator()?
            .allocate::<Vector4<f32>>("positions_out", positions_in.len::<Vector4<f32>>())?;
        let position_gradients_out = context.allocator()?.allocate::<Matrix4x3<f32>>(
            "position_gradients_out",
            position_gradients_in.len::<Matrix4x3<f32>>(),
        )?;
        let velocities_out = context
            .allocator()?
            .allocate::<Vector4<f32>>("velocities_out", velocities_in.len::<Vector4<f32>>())?;
        let velocity_gradients_out = context.allocator()?.allocate::<Matrix4x3<f32>>(
            "velocity_gradients_out",
            velocity_gradients_in.len::<Matrix4x3<f32>>(),
        )?;

        let mut compute_pass = encoder.begin_compute_pass(self.permute_particles.label);
        compute_pass.set_pipeline(&self.permute_particles.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.permute_particles,
                [
                    indirect.binding(),
                    permutation.binding(),
                    indices_in.binding(),
                    masses_in.binding(),
                    initial_volumes_in.binding(),
                    parameters_in.binding(),
                    positions_in.binding(),
                    position_gradients_in.binding(),
                    velocities_in.binding(),
                    velocity_gradients_in.binding(),
                    indices_out.binding(),
                    masses_out.binding(),
                    initial_volumes_out.binding(),
                    parameters_out.binding(),
                    positions_out.binding(),
                    position_gradients_out.binding(),
                    velocities_out.binding(),
                    velocity_gradients_out.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());

        Ok(Output {
            indices_out,
            masses_out,
            initial_volumes_out,
            parameters_out,
            positions_out,
            position_gradients_out,
            velocities_out,
            velocity_gradients_out,
        })
    }
}
