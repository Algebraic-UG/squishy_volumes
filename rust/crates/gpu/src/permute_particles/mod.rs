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
    pub positions_in: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        workgroup_size: NonZeroU32,
        dispatch_limit: NonZeroU32,
        permutation: &[u32],
        indices: &[u32],
        positions: &[Vector4<f32>],
    ) -> Self {
        assert_eq!(permutation.len(), positions.len());
        assert_eq!(permutation.len(), indices.len());
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: permutation.len() as u32,
        });

        let permutation = Allocation::new(device, "permutation", permutation);
        let indices_in = Allocation::new(device, "indices_in", indices);
        let positions_in = Allocation::new(device, "positions_in", positions);
        let indirect = Allocation::new(device, "indirect", &[indirect]);

        Self {
            indirect,
            permutation,
            indices_in,
            positions_in,
        }
    }
}

pub struct Output {
    pub indices_out: Allocation,
    pub positions_out: Allocation,
}

impl PipelinePart for PermuteParticles {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &GpuContext, settings: Settings) -> Self {
        let device = context.device();

        let_compiled_module!(
            permute_particles,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),
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
            positions_in,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        assert_eq!(permutation.len::<u32>(), indices_in.len::<u32>());
        assert_eq!(permutation.len::<u32>(), positions_in.len::<Vector4<f32>>());

        let indices_out = context
            .allocator()?
            .allocate::<u32>("indices_out", indices_in.len::<u32>())?;
        let positions_out = context
            .allocator()?
            .allocate::<Vector4<f32>>("positions_out", positions_in.len::<Vector4<f32>>())?;

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
                    positions_in.binding(),
                    indices_out.binding(),
                    positions_out.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());

        Ok(Output {
            indices_out,
            positions_out,
        })
    }
}
