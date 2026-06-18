// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use super::*;

pub struct MeldGrid {}

#[derive(Clone)]
pub struct Settings {}

pub struct Parameters;

pub struct Input {}

#[derive(Clone)]
pub struct InputData {}

impl Input {
    pub fn new(device: &wgpu::Device, InputData {}: InputData) -> Self {
        Self {}
    }
}

pub struct Output {}

pub struct OutputData {}

impl PipelinePart for MeldGrid {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &GpuContext, Settings {}: Settings) -> Self {
        Self {}
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {}: Input,
        Parameters {}: Parameters,
    ) -> Result<Output, GpuError> {
        Ok(Output {})
    }
}
