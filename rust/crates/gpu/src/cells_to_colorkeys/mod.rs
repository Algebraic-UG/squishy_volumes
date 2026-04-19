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

pub struct CellsToColorkeys {
    cells_to_colorkeys: CompiledModule,
}
pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub cell_ids: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        workgroup_size: NonZeroU32,
        dispatch_limit: NonZeroU32,
        cell_ids: &[Vector4<i32>],
    ) -> Self {
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: cell_ids.len() as u32,
        });

        let cell_ids = Allocation::new(device, "cell_ids", cell_ids);
        let indirect = Allocation::new(device, "indirect", &[indirect]);

        Self { indirect, cell_ids }
    }
}

pub struct Output {
    pub keys: Allocation,
}

impl PipelinePart for CellsToColorkeys {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &GpuContext, Settings { workgroup_size }: Settings) -> Self {
        let device = context.device();

        let_compiled_module!(
            cells_to_colorkeys,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (Vector4::<i32>::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64)]
            }
        );
        Self { cells_to_colorkeys }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input { indirect, cell_ids }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let keys = context
            .allocator()?
            .allocate::<u32>("keys", cell_ids.len::<Vector4<i32>>())?;

        let mut compute_pass = encoder.begin_compute_pass(self.cells_to_colorkeys.label);
        compute_pass.set_pipeline(&self.cells_to_colorkeys.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.cells_to_colorkeys,
                [indirect.binding(), cell_ids.binding(), keys.binding()],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());

        Ok(Output { keys })
    }
}
