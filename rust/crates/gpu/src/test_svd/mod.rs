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

use nalgebra::Matrix4x3;

use super::*;

pub struct TestSvd {
    test_svd: CompiledModule,
    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub matrices: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        matrices: &[Matrix4x3<f32>],
    ) -> Result<Self, GpuAllocatorError> {
        let matrices = Allocation::new(device, "matrices", matrices)?;

        Ok(Self { matrices })
    }
}

pub struct Output {
    pub svds: Allocation,
}

impl PipelinePart for TestSvd {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
    ) -> Self {
        let_compiled_module!(
            test_svd,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),
                    (Svd::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64)]
            }
        );

        Self {
            test_svd,
            workgroup_size,
            dispatch_limit,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input { matrices }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let num_matrices = matrices.len::<Matrix4x3<f32>>();
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: num_matrices.get() as u32,
        })
        .direct();

        let svds = context.allocator()?.allocate::<Svd>("svds", num_matrices)?;

        context
            .enter_module(
                encoder,
                &self.test_svd,
                [matrices.binding(), svds.binding()],
            )
            .dispatch_workgroups(x, y, z);

        Ok(Output { svds })
    }
}
