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
    #[error("Gpu error")]
    GpuError(#[from] squishy_volumes_gpu::GpuError),

    #[error("Harness error")]
    HarnessError(#[from] squishy_volumes_xpu::HarnessError),

    #[error("Frame input error")]
    FrameInputError(#[from] squishy_volumes_xpu::FrameInputError),

    #[error("Cpu compute error")]
    CpuCompute(#[from] squishy_volumes_cpu::Error),

    #[error("'{object_name}': Failed to interpret input bulk '{attribute}'")]
    InputBulkError {
        object_name: String,
        attribute: String,
        #[source]
        error: crate::InputBulkError,
    },

    #[error("The last input frame was not completed")]
    LeftoverInputFrame,

    #[error("Cannot create a new simulation without recorded input ready")]
    MissingInput,
    #[error("No frame has started for recording")]
    NoFrameStarted,
    #[error("Failed to lock directory")]
    DirectoryLockingError(#[from] squishy_volumes_directory_lock::DirectoryLockingError),

    #[error("Failed to start input recording")]
    StartInputWriting(#[source] squishy_volumes_file_input::InputError),
    #[error("Failed to record frame")]
    RecordFrame(#[source] squishy_volumes_file_input::InputError),
    #[error("Failed to finalize input")]
    FinalizingInput(#[source] squishy_volumes_file_input::InputError),
    #[error("Failed to query size")]
    QuerySize(#[source] squishy_volumes_file_input::InputError),
    #[error("Failed to start input reading")]
    StartInputReading(#[source] squishy_volumes_file_input::InputError),
    #[error("Failed to read input header")]
    ReadHeader(#[source] squishy_volumes_file_input::InputError),

    #[error("Failed to encode input header")]
    EncodingInputHeader(#[source] serde_json::Error),
    #[error("Failed to encode poll report")]
    EncodingReport(#[source] serde_json::Error),
    #[error("Failed to encode attribute")]
    EncodingAttribute(#[source] serde_json::Error),
    #[error("Failed to encode stats")]
    EncodingStats(#[source] serde_json::Error),

    #[error("Cache creation failed")]
    CacheCreation(#[source] squishy_volumes_cache::CacheError),
    #[error("Cache check failed")]
    CacheCheck(#[source] squishy_volumes_cache::CacheError),
    #[error("Failed to fetch frame")]
    CacheFetch(#[source] squishy_volumes_cache::CacheReadingError),
    #[error("Failed to fetch node count")]
    CacheNodeCount(#[source] squishy_volumes_cache::CacheError),
    #[error("Failed to drop frame")]
    CacheDropFrames(#[source] squishy_volumes_cache::CacheError),

    #[error("Failed to fetch attribute")]
    AttributeError(#[from] crate::attributes::AttributeError),

    #[error("Failed to parse input header")]
    ParsingInputHeader(#[source] serde_json::Error),
    #[error("Failed to parse frame start")]
    ParsingFrameStart(#[source] serde_json::Error),
    #[error("Failed to parse bulk meta")]
    ParsingBulkMeta(#[source] serde_json::Error),
    #[error("Failed to parse compute settings")]
    ParsingComputeSettings(#[source] serde_json::Error),
    #[error("Failed to parse attribute")]
    ParseAttribute(#[source] serde_json::Error),

    #[error("The allowed disk space of {0} bytes was exceeded while recording inputs.")]
    DiskSpaceExceededWhileRecording(u64),

    #[error("Something went really wrong and the compute stats mutex is poisoned")]
    ComputeStatsMutexPoisoned,

    #[error("Failed to create initial state")]
    InitializationError(#[from] StateInitializationError),
    #[error("Failed to store frame")]
    StoreError(#[source] squishy_volumes_cache::CacheError),

    #[error("Something went really wrong and the compute thread paniced: {0}")]
    ComputePanic(String),
}
