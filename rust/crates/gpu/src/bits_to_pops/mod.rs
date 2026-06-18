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

use super::*;

pub struct BitsToPops {
    bits_to_pops: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub bits: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings { workgroup_size }: Settings,
        dispatch_limit: NonZeroU32,
        bits: &[u32],
    ) -> Self {
        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: bits.len() as u32,
        });

        let indirect = Allocation::new(device, "indirect", &[indirect]);
        let bits = Allocation::new(device, "bits", bits);

        Self { indirect, bits }
    }
}

pub struct Output {
    pub pops: Allocation,
}

impl PipelinePart for BitsToPops {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &GpuContext, Settings { workgroup_size }: Settings) -> Self {
        let device = context.device();

        let_compiled_module!(
            bits_to_pops,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64)]
            }
        );

        Self { bits_to_pops }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input { indirect, bits }: Input,
        _: Self::Parameters,
    ) -> Result<Output, GpuError> {
        let pops = context
            .allocator()?
            .allocate::<u32>("pops", bits.len::<u32>())?;

        let mut compute_pass = encoder.begin_compute_pass(self.bits_to_pops.label);
        compute_pass.set_pipeline(&self.bits_to_pops.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.bits_to_pops,
                [indirect.binding(), bits.binding(), pops.binding()],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());
        drop(compute_pass);

        Ok(Output { pops })
    }
}
