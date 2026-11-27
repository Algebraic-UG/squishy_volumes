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

use squishy_volumes_file_frame::IoStateError;

use crate::{AllocatorError, ProfilerError, ShaderError};

#[allow(dead_code)] // fields are read in the error below
#[derive(Debug, Clone)]
pub struct ExceedingLimit {
    pub name: &'static str,
    pub required: u64,
    pub allowed: u64,
}

#[derive(thiserror::Error, Debug)]
pub enum ContextError {
    #[error("failed to find adapter '{requested}' (available are: {available:?})")]
    AdapterNotFound {
        requested: String,
        available: Vec<String>,
    },

    #[error("failed to request the adapter")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),

    #[error(
        "the adapter allows only {allowed} workgroups per dispatch dimension, but {required} are required"
    )]
    SmallMaxWorkgroupsPerDimension { allowed: u32, required: u32 },

    #[error("the adapter reports a subgroup size of zero")]
    SubgroupSizeZero,

    #[error("adapter does not support compute shaders")]
    ComputeNotSupported,

    #[error("adapter is missing required features: {0}")]
    MissingRequiredFeatures(wgpu::Features),

    #[error("exceeding adapter's limits: {0:?}")]
    ExceedingRequiredLimits(Vec<ExceedingLimit>),

    #[error("failed to request the device")]
    RequestDevice(#[from] wgpu::RequestDeviceError),

    #[error("failed to allocate the status binding")]
    AllocateStatus(#[source] AllocatorError),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io state error")]
    IoState(#[from] IoStateError),

    #[error("frame input error")]
    FrameInput(#[from] squishy_volumes_xpu::FrameInputError),

    #[error(transparent)]
    Canceled(#[from] squishy_volumes_xpu::Canceled),

    #[error("harness error")]
    Harness(#[from] squishy_volumes_xpu::HarnessError),

    #[error("failed to set up the gpu")]
    Context(#[from] ContextError),

    #[error("failed to map {label} for downloading")]
    MapRange {
        label: &'static str,
        source: wgpu::MapRangeError,
    },

    #[error("exceeding the theoretical maximum of grid nodes, most likely encountered a bug")]
    MaxGridNodesExceeded,

    #[error("failed to create the gpu pipeline")]
    PipelineCreation(#[from] PipelineError),

    #[error("failed to create the profiling csv")]
    CreateProfilingCsv(#[source] ProfilerError),
    #[error("failed to collect the profiling data")]
    CollectProfilingData(#[source] ProfilerError),
    #[error("failed to write the profiling csv")]
    WriteProfilingCsv(#[source] ProfilerError),

    #[error("failed to wait for the gpu")]
    Poll(#[from] wgpu::PollError),

    #[error("failed to allocate a binding")]
    Allocation(#[from] AllocatorError),

    #[error("allocator is missing")]
    AllocatorMissing,

    #[error("indirect allocator is missing")]
    IndirectAllocatorMissing,

    #[error("there's no shader registered to {0}")]
    ShaderIdMissing(u32),

    #[error("shader {reporting_shader} reported a failure")]
    Shader {
        reporting_shader: &'static str,
        source: ShaderError,
    },

    #[error("input to the gpu wasn't valid")]
    Input(#[from] InputError),
}

#[derive(thiserror::Error, Debug)]
pub enum PipelineError {
    #[error("could not determine subgroup size for {label}")]
    FailedToDetermineSubgroupSize { label: &'static str },

    #[error("subgroup size mismatch, '{a_label}' has {a_size}, but '{b_label}' has {b_size}")]
    SubgroupSizeMismatch {
        a_label: &'static str,
        a_size: u32,
        b_label: &'static str,
        b_size: u32,
    },

    #[error(
        "{label}: the workgroup size {workgroup_size} isn't a multiple of the subgroup size {subgroup_size}"
    )]
    WorkgroupSizeNotMultipleOfSubgroupSize {
        label: &'static str,
        workgroup_size: u32,
        subgroup_size: u32,
    },

    #[error("{label}: the subgroup size {subgroup_size} is too small, needed at least {needed}")]
    SubgroupSizeTooSmall {
        label: &'static str,
        subgroup_size: u32,
        needed: u32,
    },

    #[error("failed to set pipeline constants")]
    PipelineConstant(#[from] wgpu::naga::back::pipeline_constants::PipelineConstantError),

    #[error("duplicate constant: {0}")]
    DuplicateConstant(&'static str),
}

#[derive(thiserror::Error, Debug)]
pub enum InputError {
    #[error(
        "range does not fit: '{a}' has length {a_len} and won't fit with offset {offset} into '{b}' (length {b_len})"
    )]
    RangeDoesNotFit {
        a: &'static str,
        a_len: usize,
        b: &'static str,
        b_len: usize,
        offset: usize,
    },
    #[error("length mismatch: '{a}' has length {a_len} but '{b}' has length {b_len}")]
    LengthMismatch {
        a: &'static str,
        a_len: usize,
        b: &'static str,
        b_len: usize,
    },

    #[error(
        "length multiple mismatch: '{a}' has length {a_len} but '{b}' has length {b_len}, multiple {multiple}"
    )]
    LengthMultipleMismatch {
        a: &'static str,
        a_len: usize,
        b: &'static str,
        b_len: usize,
        multiple: usize,
    },
    #[error(
        "index out of bounds: '{indices}' contains {index} but length of {indexed} is {indexed_len}"
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
            Err($crate::InputError::LengthMismatch {
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
macro_rules! check_range {
    ($a:expr, $b:expr, $offset:expr) => {
        if $a.len() > $b.len() {
            Err($crate::InputError::RangeDoesNotFit {
                a: stringify!($a),
                a_len: $a.len(),
                b: stringify!($b),
                b_len: $b.len(),
                offset: $offset,
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
            Err($crate::InputError::LengthMultipleMismatch {
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
            Err($crate::InputError::IndexOutOfBounds {
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
