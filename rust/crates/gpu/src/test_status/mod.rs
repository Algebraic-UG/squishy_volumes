// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#[cfg(test)]
mod test;

use std::num::NonZeroU32;

use super::*;

pub struct TestStatus {
    test_status: CompiledModule,
}

pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input;

pub struct Output;

impl PipelinePart for TestStatus {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings { workgroup_size }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            test_status,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [],
                immediate_size: 0,
                constants: [],
            }
        );

        Ok(Self { test_status })
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
