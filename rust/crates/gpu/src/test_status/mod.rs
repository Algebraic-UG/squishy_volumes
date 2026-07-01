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

pub struct TestStatus {
    test_status: CompiledModule,
}

pub struct Settings;

pub struct Parameters;

pub struct Input;

pub struct Output;

impl PipelinePart for TestStatus {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &mut GpuContext, _: Settings) -> Self {
        let_compiled_module!(
            test_status,
            CompiledModuleSettings {
                context,
                bind_group_entries: [],
                immediate_size: 0,
                constants: [],
            }
        );

        Self { test_status }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        _: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        context
            .enter_module(encoder, &self.test_status, [])
            .dispatch_workgroups(1, 1, 1);

        Ok(Output)
    }
}
