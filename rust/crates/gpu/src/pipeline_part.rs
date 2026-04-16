// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use crate::{GpuAllocator, GpuContext, GpuError};

pub trait PipelinePart {
    type Settings;
    type Parameters;

    type Input;
    type Output;

    fn new(context: &GpuContext, settings: Self::Settings) -> Self;

    fn compute_in_pass(
        &self,
        context: &GpuContext,
        allocator: &mut GpuAllocator,
        compute_pass: &mut wgpu::ComputePass,
        input: Self::Input,
        parameters: Self::Parameters,
    ) -> Result<Self::Output, GpuError>;
}
