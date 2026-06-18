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

pub struct NodeIdsToMurmur {
    node_ids_to_murmur: CompiledModule,
}

pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub cells: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        workgroup_size: NonZeroU32,
        dispatch_limit: NonZeroU32,
        cells: &[Vector4<i32>],
    ) -> Self {
        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: cells.len() as u32,
        });

        let cells = Allocation::new(device, "cells", cells);
        let indirect = Allocation::new(device, "indirect", &[indirect]);

        Self { indirect, cells }
    }
}

pub struct Output {
    pub hashes: Allocation,
}

impl PipelinePart for NodeIdsToMurmur {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &GpuContext, Settings { workgroup_size }: Settings) -> Self {
        let device = context.device();

        let_compiled_module!(
            node_ids_to_murmur,
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

        Self { node_ids_to_murmur }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input { indirect, cells }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let hashes = context
            .allocator()?
            .allocate::<u32>("hashes", cells.len::<Vector4<i32>>())?;

        let mut compute_pass = encoder.begin_compute_pass(self.node_ids_to_murmur.label);
        compute_pass.set_pipeline(&self.node_ids_to_murmur.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.node_ids_to_murmur,
                [indirect.binding(), cells.binding(), hashes.binding()],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());
        Ok(Output { hashes })
    }
}
