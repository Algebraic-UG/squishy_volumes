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

    #[error("Cpu compute error")]
    CpuCompute(#[from] squishy_volumes_cpu::Error),

    #[error("Cannot create a new simulation without recorded input ready")]
    MissingInput,
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
    #[error("Failed to start input reading")]
    StartInputReading(#[source] squishy_volumes_file_input::InputError),

    #[error("Failed to encode input header")]
    EncodingInputHeader(#[source] serde_json::Error),
    #[error("Failed to encode poll report")]
    EncodingReport(#[source] serde_json::Error),

    #[error("Cache creation failed")]
    CacheCreation(#[source] squishy_volumes_cache::CacheError),
    #[error("Cache check failed")]
    CacheCheck(#[source] squishy_volumes_cache::CacheError),
    #[error("Failed to fetch frame")]
    CacheFetch(#[source] squishy_volumes_cache::CacheReadingError),
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

    #[error("The allowed disk space of {0} bytes was exceeded while recording inputs.")]
    DiskSpaceExceededWhileRecording(u64),

    #[error("Failed to create initial state")]
    InitializationError(#[from] StateInitializationError),
    #[error("Failed to store frame")]
    StoreError(#[source] squishy_volumes_cache::CacheError),

    #[error("Internal error (panic) in the {thread} thread: {message}, please report this bug")]
    Panic {
        thread: &'static str,
        message: String,
    },
    #[error(
        "This simulation can't be used anymore after an internal error (panic: {0}).\nPlease reload it."
    )]
    BrokenSimulation(String),
    #[error(
        "This input recording can't be used anymore after an internal error (panic: {0}).\nPlease record the input again."
    )]
    BrokenSimulationInput(String),
}

impl Error {
    // Cancellation hides behind many wrappers, see
    // https://github.com/Algebraic-UG/squishy_volumes/issues/446
    pub fn is_cancellation(&self) -> bool {
        std::iter::successors(Some(self as &dyn std::error::Error), |error| error.source()).any(
            |error| {
                matches!(
                    error.downcast_ref(),
                    Some(squishy_volumes_xpu::HarnessError::Canceled)
                )
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use squishy_volumes_xpu::HarnessError;

    use super::*;

    #[test]
    fn recognizes_wrapped_cancellations() {
        let cancellations = [
            Error::HarnessError(HarnessError::Canceled),
            Error::CpuCompute(squishy_volumes_cpu::Error::HarnessError(
                HarnessError::Canceled,
            )),
            Error::GpuError(squishy_volumes_gpu::GpuError::HarnessError(
                HarnessError::Canceled,
            )),
            Error::InitializationError(StateInitializationError::HarnessError(
                HarnessError::Canceled,
            )),
        ];
        for cancellation in cancellations {
            assert!(cancellation.is_cancellation(), "{cancellation:?}");
        }
    }

    #[test]
    fn other_errors_are_not_cancellations() {
        let errors = [
            Error::StoredError("broken".to_string()),
            Error::Panic {
                thread: "compute",
                message: "broken".to_string(),
            },
            Error::CpuCompute(squishy_volumes_cpu::Error::ZeroTimeStep),
            Error::GpuError(squishy_volumes_gpu::GpuError::NoParticles),
        ];
        for error in errors {
            assert!(!error.is_cancellation(), "{error:?}");
        }
    }
}
