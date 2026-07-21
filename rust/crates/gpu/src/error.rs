// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use thiserror::Error;

use crate::{GpuAllocatorError, GpuShaderError, ProfilerError};

#[allow(dead_code)] // fields are read in the error below
#[derive(Debug, Clone)]
pub struct ExceedingLimit {
    pub name: &'static str,
    pub required: u64,
    pub allowed: u64,
}

#[derive(Error, Debug)]
pub enum GpuError {
    #[error("Failed to find adapter '{requested}' (available are: {available:?})")]
    AdapterNotFound {
        requested: String,
        available: Vec<String>,
    },

    #[error("Failed to map {label}: {error}")]
    MapRangeError {
        label: &'static str,
        error: wgpu::MapRangeError,
    },

    #[error("Exceeding the theorical maximum of grid nodes, most likely encountered a bug.")]
    MaxGridNodesExceeded,

    #[error("Failed to create GPU pipeline: {0}")]
    PipelineCreation(#[from] GpuPipelineCreationError),

    #[error("Csv profiling failed: {0}")]
    CsvProfilerError(#[from] ProfilerError),

    #[error("Poll error: {0}")]
    PollError(#[from] wgpu::PollError),

    #[error("Harness error: {0}")]
    HarnessError(#[from] squishy_volumes_xpu::HarnessError),

    #[error("Frame input error: {0}")]
    FrameInputError(#[from] squishy_volumes_xpu::FrameInputError),

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
pub enum GpuPipelineCreationError {
    #[error("Could not determine subgroup size for {label}")]
    FailedToDetermineSubgroupSize { label: &'static str },

    #[error("Subgroup size mismatch, '{a_label}' has {a_size}, but '{b_label}' has {b_size}")]
    SubgroupSizeMismatch {
        a_label: &'static str,
        a_size: u32,
        b_label: &'static str,
        b_size: u32,
    },

    #[error(
        "{label}: The workgroup size {workgroup_size} isn't a multiple of the subgroup size {subgroup_size}"
    )]
    WorkgroupSizeNotMultipleOfSubgroupSize {
        label: &'static str,
        workgroup_size: u32,
        subgroup_size: u32,
    },

    #[error("{label}: The subgroup size {subgroup_size} is too small, needed at least {needed}.")]
    SubgroupSizeTooSmall {
        label: &'static str,
        subgroup_size: u32,
        needed: u32,
    },
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
