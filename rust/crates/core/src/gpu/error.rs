// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use thiserror::Error;

#[derive(Debug)]
pub struct ExceedingLimit {
    pub name: &'static str,
    pub required: u64,
    pub allowed: u64,
}

#[derive(Error, Debug)]
pub enum GpuError {
    #[error("Failed to request the adapter: {0}")]
    RequestAdapterError(#[from] wgpu::RequestAdapterError),

    #[error("We can not deal with variable subgroup size yet.")]
    VariableSubgroupSize,

    #[error("Subgroup size is zero.")]
    SubgroupSizeZero,

    #[error("Adapter does not support compute shaders.")]
    ComputeNotSupported,

    #[error("Adapter is missing required features: {0}")]
    MissingRequiredFeatures(wgpu::Features),

    #[error("Exceeding adapter's limits: {0:?}")]
    ExceedingRequiredLimits(Vec<ExceedingLimit>),

    #[error("Failed to request the device: {0}")]
    RequestDeviceError(#[from] wgpu::RequestDeviceError),
}
