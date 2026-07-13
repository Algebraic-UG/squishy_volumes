// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use crate::initialization::StateInitializationError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Gpu error: {0}")]
    GpuError(#[from] squishy_volumes_gpu::GpuError),

    #[error("Harness error: {0}")]
    HarnessError(#[from] squishy_volumes_xpu::HarnessError),

    #[error("Frame input error: {0}")]
    FrameInputError(#[from] squishy_volumes_xpu::FrameInputError),

    #[error("Cpu compute error: {0}")]
    CpuCompute(#[from] squishy_volumes_cpu::Error),

    #[error("'{object_name}': Failed to interpret input bulk '{attribute}': {error}")]
    InputBulkError {
        object_name: String,
        attribute: String,
        error: crate::InputBulkError,
    },

    #[error("The last input frame was not completed")]
    LeftoverInputFrame,

    #[error("Cannot create a new simulation without recorded input ready")]
    MissingInput,
    #[error("No frame has started for recording")]
    NoFrameStarted,
    #[error("Failed to lock directory: {0}")]
    DirectoryLockingError(#[from] squishy_volumes_directory_lock::DirectoryLockingError),

    #[error("Failed to start input recording: {0}")]
    StartInputWriting(squishy_volumes_file_input::InputError),
    #[error("Failed to record frame: {0}")]
    RecordFrame(squishy_volumes_file_input::InputError),
    #[error("Failed to finalize input: {0}")]
    FinalizingInput(squishy_volumes_file_input::InputError),
    #[error("Failed to query size: {0}")]
    QuerySize(squishy_volumes_file_input::InputError),
    #[error("Failed to start input reading: {0}")]
    StartInputReading(squishy_volumes_file_input::InputError),
    #[error("Failed to read input header: {0}")]
    ReadHeader(squishy_volumes_file_input::InputError),

    #[error("Failed to encode input header: {0}")]
    EncodingInputHeader(serde_json::Error),
    #[error("Failed to encode poll report: {0}")]
    EncodingReport(serde_json::Error),
    #[error("Failed to encode attribute: {0}")]
    EncodingAttribute(serde_json::Error),
    #[error("Failed to encode stats: {0}")]
    EncodingStats(serde_json::Error),

    #[error("Cache creation failed: {0}")]
    CacheCreation(squishy_volumes_cache::CacheError),
    #[error("Cache check failed: {0}")]
    CacheCheck(squishy_volumes_cache::CacheError),
    #[error("Failed to fetch frame: {0}")]
    CacheFetch(squishy_volumes_cache::CacheReadingError),
    #[error("Failed to fetch node count: {0}")]
    CacheNodeCount(squishy_volumes_cache::CacheError),
    #[error("Failed to drop frame: {0}")]
    CacheDropFrames(squishy_volumes_cache::CacheError),

    #[error("Failed to fetch attribute: {0}")]
    AttributeError(#[from] crate::attributes::AttributeError),

    #[error("Failed to parse input header: {0}")]
    ParsingInputHeader(serde_json::Error),
    #[error("Failed to parse frame start: {0}")]
    ParsingFrameStart(serde_json::Error),
    #[error("Failed to parse bulk meta: {0}")]
    ParsingBulkMeta(serde_json::Error),
    #[error("Failed to parse compute settings: {0}")]
    ParsingComputeSettings(serde_json::Error),
    #[error("Failed to parse attribute: {0}")]
    ParseAttribute(serde_json::Error),

    #[error("The allowed disk space of {0} bytes was exceeded while recording inputs.")]
    DiskSpaceExceededWhileRecording(u64),

    #[error("Something went really wrong and the compute stats mutex is poisoned")]
    ComputeStatsMutexPoisoned,

    #[error("Failed to create initial state")]
    InitializationError(#[from] StateInitializationError),
    #[error("Failed to store frame: {0}")]
    StoreError(squishy_volumes_cache::CacheError),

    #[error("Something went really wrong and the compute thread paniced")]
    ComputePanic,
}
