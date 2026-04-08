// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use crate::GpuContext;

pub trait PipelinePart {
    type Settings;
    type Parameters;

    type BufferInput<'a>;
    type Buffers;
    type BufferBindings<'a>;

    fn new(context: &GpuContext, settings: Self::Settings) -> Self;
    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        input: Self::BufferInput<'a>,
    ) -> Self::Buffers;
    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        buffer_bindings: Self::BufferBindings<'a>,
        parameters: Self::Parameters,
    );
}
