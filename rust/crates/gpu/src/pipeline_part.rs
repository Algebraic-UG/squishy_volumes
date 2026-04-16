// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::ops::{Deref, DerefMut};

use crate::{GpuContext, GpuError};

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
            CommandEncoder::Encoder(command_encoder) => ComputePass::Pass(
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label,
                    ..Default::default()
                }),
            ),
            CommandEncoder::Scoped(scope) => {
                ComputePass::Scoped(scope.scoped_compute_pass(label.unwrap_or("unlabled")))
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

    fn new(context: &GpuContext, settings: Self::Settings) -> Self;

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: CommandEncoder,
        input: Self::Input,
        parameters: Self::Parameters,
    ) -> Result<Self::Output, GpuError>;
}
