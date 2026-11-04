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

#[derive(thiserror::Error, Debug)]
pub enum CacheError {
    #[error("An error occured while writing to cache")]
    Writing(#[from] CacheWritingError),
    #[error("An error occured while reading the cache")]
    Reading(#[from] CacheReadingError),
    #[error("An error occured while clearing old frames")]
    Cleanup(#[from] CacheCleanupError),
    #[error("Something went really wrong and the store thread mutex is poisoned")]
    StoreThreadLockPoisoned,
}

#[derive(thiserror::Error, Debug)]
pub enum CacheWritingError {
    #[error("Failed to create output frame")]
    CreateFrame(#[source] std::io::Error),
    #[error("Failed to write output frame")]
    WriteFrame(#[source] std::io::Error),
    #[error("Failed to move output frame")]
    MoveFrame(#[source] std::io::Error),
    #[error("Failed to write stats")]
    WriteStats(#[source] std::io::Error),
    #[error("Failed to serialize state")]
    Serialization(#[from] squishy_volumes_file_frame::Error),
    #[error("Failed to forward output frame to writing thread")]
    Sending,
    #[error("Store thread is gone")]
    ThreadGone,
    #[error("Store thread stopped")]
    ThreadStopped,
    #[error("Exceeding allowed disk space")]
    ExceedingSpace,
    #[error("Unknown io error")]
    IoError(#[from] std::io::Error),
    #[error("Something went really wrong and the store thread paniced")]
    StoreThreadPaniced,
}

#[derive(thiserror::Error, Debug)]
pub enum CacheReadingError {
    #[error("Something went really wrong and the loaded frame mutex is poisoned")]
    LoadedFrameLockPoisoned,
    #[error("Some frames are missing from the sequence")]
    FrameSequenceBroken,
    #[error("Frame is not computed yet")]
    FrameNotReady,
    #[error("Failed to read frame")]
    ReadFrame(#[source] std::io::Error),
    #[error("Failed to deserialize state")]
    Deserialization(#[from] squishy_volumes_file_frame::Error),
    #[error("Unknown io error")]
    IoError(#[from] std::io::Error),
}
#[derive(thiserror::Error, Debug)]
pub enum CacheCleanupError {
    #[error(
        "Failed to remove frame {frame} at {path:?}, it might still be open in another program (e.g. Blender displaying it)"
    )]
    RemoveFrame {
        frame: usize,
        path: std::path::PathBuf,
        #[source]
        error: std::io::Error,
    },
    #[error("Unknown io error")]
    IoError(#[from] std::io::Error),
}
