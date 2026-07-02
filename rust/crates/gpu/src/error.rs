// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use thiserror::Error;

use crate::{GpuAllocatorError, GpuShaderError};

#[allow(dead_code)] // fields are read in the error below
#[derive(Debug, Clone)]
pub struct ExceedingLimit {
    pub name: &'static str,
    pub required: u64,
    pub allowed: u64,
}

#[derive(Error, Debug)]
pub enum GpuError {
    #[error("No particle input")]
    NoParticles,

    #[error("Failed to request the adapter: {0}")]
    RequestAdapterError(#[from] wgpu::RequestAdapterError),

    #[error("We can not deal with variable subgroup size yet.")]
    VariableSubgroupSize,

    #[error("We can not deal with a small max workgroup dispatch yet.")]
    SmallMaxWorkGroupPerDimension,

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

    #[error("Failed to allocate a binding: {0}")]
    AllocationError(#[from] GpuAllocatorError),

    #[error("Allocator is missing")]
    AllocatorMissing,

    #[error("Indirect allocator is missing")]
    IndirectAllocatorMissing,

    #[error("Something went wrong with creating the profiler: {0}")]
    ProfilerError(#[from] wgpu_profiler::CreationError),

    #[error("There's no shader registered to {0}")]
    ShaderIdMissing(u32),

    #[error("A shader reported errors: {0}")]
    Shader(#[from] GpuShaderError),

    #[error("Input to the GPU wasn't valid: {0}")]
    Input(#[from] GpuInputError),
}

#[derive(Error, Debug)]
pub enum GpuInputError {
    #[error("Length mismatch: '{a}' has length {a_len} but '{b}' has length {b_len}")]
    LengthMismatch {
        a: &'static str,
        a_len: usize,
        b: &'static str,
        b_len: usize,
    },
    #[error(
        "Length multiple mismatch: '{a}' has length {a_len} but '{b}' has length {b_len}, multiple {multiple}"
    )]
    LengthMultipleMismatch {
        a: &'static str,
        a_len: usize,
        b: &'static str,
        b_len: usize,
        multiple: usize,
    },
    #[error(
        "Index out of bounds: '{indices}' contains {index} but length of {indexed} is {indexed_len}"
    )]
    IndexOutOfBounds {
        indices: &'static str,
        index: usize,
        indexed: &'static str,
        indexed_len: usize,
    },
}

#[macro_export]
macro_rules! check_length {
    ($a:expr, $b:expr) => {
        if $a.len() != $b.len() {
            Err(GpuInputError::LengthMismatch {
                a: stringify!($a),
                a_len: $a.len(),
                b: stringify!($b),
                b_len: $b.len(),
            })
        } else {
            Ok(())
        }
    };
}

#[macro_export]
macro_rules! check_length_multiple {
    ($a:expr, $b:expr, $multiple:expr) => {
        if $a.len() != $b.len() * $multiple {
            Err(GpuInputError::LengthMultipleMismatch {
                a: stringify!($a),
                a_len: $a.len(),
                b: stringify!($b),
                b_len: $b.len(),
                multiple: $multiple,
            })
        } else {
            Ok(())
        }
    };
}

#[macro_export]
macro_rules! check_indices_valid {
    ($indices:expr, $indexed:expr) => {
        if let Some(&index) = $indices
            .into_iter()
            .find(|&&index| (index as usize) >= $indexed.len())
        {
            Err(GpuInputError::IndexOutOfBounds {
                indices: stringify!($indices),
                index: index as usize,
                indexed: stringify!($indexed),
                indexed_len: $indexed.len(),
            })
        } else {
            Ok(())
        }
    };
}
