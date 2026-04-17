// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU32;

use nalgebra::Vector4;

use super::*;

#[cfg(test)]
mod test;

pub struct PositionsToKeys {
    positions_to_keys: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub cell_size: f32,
}

pub struct Parameters {
    pub dimension: u32,
}

pub struct Input {
    pub indirect: Allocation,
    pub positions: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        workgroup_size: NonZeroU32,
        dispatch_limit: NonZeroU32,
        positions: &[Vector4<f32>],
    ) -> Self {
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: positions.len() as u32,
        });

        let positions = Allocation::new(device, "positions", positions);
        let indirect = Allocation::new(device, "indirect", &[indirect]);

        Self {
            indirect,
            positions,
        }
    }
}

pub struct Output {
    pub keys: Allocation,
}

impl PipelinePart for PositionsToKeys {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &GpuContext, settings: Self::Settings) -> Self {
        let workgroup_size = settings.workgroup_size.get();
        let cell_size = settings.cell_size;
        assert!(cell_size > 0.);

        let device = context.device();
        let_compiled_module!(
            positions_to_keys,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 4,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size as f64),
                    ("CELL_SIZE", cell_size as f64),
                ],
            }
        );

        Self { positions_to_keys }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            positions,
        }: Input,
        Parameters { dimension }: Parameters,
    ) -> Result<Output, GpuError> {
        assert!(dimension < 3);

        let keys = context
            .allocator()?
            .allocate::<u32>("keys", positions.len::<Vector4<f32>>())?;

        let mut compute_pass = encoder.begin_compute_pass(self.positions_to_keys.label);
        compute_pass.set_pipeline(&self.positions_to_keys.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.positions_to_keys,
                [indirect.binding(), positions.binding(), keys.binding()],
            ),
            &[],
        );

        compute_pass.set_immediates(0, bytemuck::bytes_of(&dimension));
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());

        Ok(Output { keys })
    }
}
