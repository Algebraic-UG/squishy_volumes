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

use std::ops::{Deref, DerefMut};

use crate::{GpuContext, GpuError, GpuPipelineCreationError};

pub enum CommandEncoder<'a> {
    Encoder(&'a mut wgpu::CommandEncoder),
    Scoped(wgpu_profiler::Scope<'a, wgpu::CommandEncoder>),
}

pub enum ComputePass<'a> {
    Pass(wgpu::ComputePass<'a>),
    Scoped(wgpu_profiler::OwningScope<'a, wgpu::ComputePass<'a>>),
}

impl CommandEncoder<'_> {
    pub fn begin_compute_pass<'a>(&'a mut self, label: Option<&'static str>) -> ComputePass<'a> {
        match self {
            CommandEncoder::Encoder(command_encoder) => ComputePass::Pass({
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label,
                    ..Default::default()
                })
            }),
            CommandEncoder::Scoped(scope) => {
                ComputePass::Scoped(scope.scoped_compute_pass(label.unwrap_or("unlabled")))
            }
        }
    }

    pub fn scope<'a>(&'a mut self, label: Option<&'static str>) -> CommandEncoder<'a> {
        match self {
            CommandEncoder::Encoder(command_encoder) => CommandEncoder::Encoder(command_encoder),
            // Note that this will do nothing without TIMESTAMP_QUERY_INSIDE_ENCODERS
            CommandEncoder::Scoped(scope) => {
                CommandEncoder::Scoped(scope.scope(label.unwrap_or("unlabled")))
            }
        }
    }
}

impl<'a> Deref for CommandEncoder<'a> {
    type Target = wgpu::CommandEncoder;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Encoder(inner) => inner,
            Self::Scoped(inner) => inner,
        }
    }
}

impl<'a> DerefMut for CommandEncoder<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Encoder(inner) => inner,
            Self::Scoped(inner) => inner,
        }
    }
}

impl<'a> From<&'a mut wgpu::CommandEncoder> for CommandEncoder<'a> {
    fn from(value: &'a mut wgpu::CommandEncoder) -> Self {
        Self::Encoder(value)
    }
}

impl<'a> From<wgpu_profiler::Scope<'a, wgpu::CommandEncoder>> for CommandEncoder<'a> {
    fn from(value: wgpu_profiler::Scope<'a, wgpu::CommandEncoder>) -> Self {
        Self::Scoped(value)
    }
}

impl<'a> Deref for ComputePass<'a> {
    type Target = wgpu::ComputePass<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Pass(inner) => inner,
            Self::Scoped(inner) => inner,
        }
    }
}

impl<'a> DerefMut for ComputePass<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Pass(inner) => inner,
            Self::Scoped(inner) => inner,
        }
    }
}

pub trait PipelinePart {
    type Settings;
    type Parameters;

    type Input;
    type Output;

    fn new(
        context: &mut GpuContext,
        settings: Self::Settings,
    ) -> Result<Self, GpuPipelineCreationError>
    where
        Self: Sized;

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        input: Self::Input,
        parameters: Self::Parameters,
    ) -> Result<Self::Output, GpuError>;
}
