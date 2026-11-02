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

use crate::initialization::StateInitializationError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "This stored frame is already broken.\nPlease pick an earlier frame to rebake from.\n\nDetails:\n{0}"
    )]
    StoredError(String),

    #[error("Gpu error")]
    GpuError(#[from] squishy_volumes_gpu::GpuError),

    #[error("Harness error")]
    HarnessError(#[from] squishy_volumes_xpu::HarnessError),

    #[error("Frame input error")]
    FrameInputError(#[from] squishy_volumes_xpu::FrameInputError),

    #[error("Cpu compute error")]
    CpuCompute(#[from] squishy_volumes_cpu::Error),

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
    #[error("Failed to start frame")]
    StartFrame(#[source] squishy_volumes_file_input::InputError),
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

    #[error("Failed to create initial state")]
    InitializationError(#[from] StateInitializationError),
    #[error("Failed to store frame")]
    StoreError(#[source] squishy_volumes_cache::CacheError),

    #[error("Something went really wrong and the compute thread paniced: {0}")]
    ComputePanic(String),
}
