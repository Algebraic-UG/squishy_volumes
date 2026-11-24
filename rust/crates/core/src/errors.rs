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

use squishy_volumes_xpu::Canceled;

use crate::initialization::InitializationError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "This stored frame is already broken.\nPlease pick an earlier frame to rebake from.\n\nDetails:\n{0}"
    )]
    StoredError(String),

    #[error(transparent)]
    Canceled(#[from] Canceled),

    #[error("gpu computation failed")]
    Gpu(#[source] squishy_volumes_gpu::Error),

    #[error("cpu computation failed")]
    Cpu(#[source] squishy_volumes_cpu::Error),

    #[error("harness error")]
    Harness(#[from] squishy_volumes_xpu::HarnessError),

    #[error("cannot create a new simulation without recorded input ready")]
    MissingInput,

    #[error("failed to lock the cache directory")]
    DirectoryLock(#[from] squishy_volumes_directory_lock::Error),

    #[error("failed to start recording the input")]
    StartRecordingInput(#[source] squishy_volumes_file_input::Error),
    #[error("failed to start an input frame")]
    StartInputFrame(#[source] squishy_volumes_file_input::Error),
    #[error("failed to record input frame data")]
    RecordInputFrame(#[source] squishy_volumes_file_input::Error),
    #[error("failed to finalize the input")]
    FinalizeInput(#[source] squishy_volumes_file_input::Error),
    #[error("failed to start reading the input")]
    StartReadingInput(#[source] squishy_volumes_file_input::Error),

    #[error("failed to encode the input header")]
    EncodeInputHeader(#[source] serde_json::Error),
    #[error("failed to encode the poll report")]
    EncodePollReport(#[source] serde_json::Error),
    #[error("failed to parse the input header")]
    ParseInputHeader(#[source] serde_json::Error),
    #[error("failed to parse the frame start")]
    ParseFrameStart(#[source] serde_json::Error),
    #[error("failed to parse the bulk meta")]
    ParseBulkMeta(#[source] serde_json::Error),
    #[error("failed to parse the compute settings")]
    ParseComputeSettings(#[source] serde_json::Error),

    #[error("failed to create the cache")]
    CreateCache(#[source] squishy_volumes_cache::Error),
    #[error("failed to check the cache")]
    CheckCache(#[source] squishy_volumes_cache::Error),
    #[error("failed to load the checkpoint frame")]
    LoadCheckpoint(#[source] squishy_volumes_cache::Error),
    #[error("failed to drop frames")]
    DropFrames(#[source] squishy_volumes_cache::Error),
    #[error("failed to store a frame")]
    StoreFrame(#[source] squishy_volumes_cache::Error),

    #[error("the allowed disk space of {max_bytes} bytes was exceeded while recording inputs")]
    DiskSpaceExceeded { max_bytes: u64 },

    #[error("failed to create the initial state")]
    Initialization(#[source] InitializationError),

    #[error("internal error (panic) in the {thread} thread: {message}, please report this bug")]
    Panic {
        thread: &'static str,
        message: String,
    },
    #[error(
        "this simulation can't be used anymore after an internal error (panic: {0}), please reload it"
    )]
    BrokenSimulation(String),
    #[error(
        "this input recording can't be used anymore after an internal error (panic: {0}), please record the input again"
    )]
    BrokenSimulationInput(String),
}

impl From<squishy_volumes_cpu::Error> for Error {
    fn from(error: squishy_volumes_cpu::Error) -> Self {
        match error {
            squishy_volumes_cpu::Error::Canceled(canceled) => Self::Canceled(canceled),
            error => Self::Cpu(error),
        }
    }
}

impl From<squishy_volumes_gpu::Error> for Error {
    fn from(error: squishy_volumes_gpu::Error) -> Self {
        match error {
            squishy_volumes_gpu::Error::Canceled(canceled) => Self::Canceled(canceled),
            error => Self::Gpu(error),
        }
    }
}

impl From<InitializationError> for Error {
    fn from(error: InitializationError) -> Self {
        match error {
            InitializationError::Canceled(canceled) => Self::Canceled(canceled),
            error => Self::Initialization(error),
        }
    }
}
