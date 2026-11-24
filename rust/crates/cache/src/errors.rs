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
pub enum Error {
    #[error("failed to write to the cache")]
    Write(#[from] WriteError),
    #[error("failed to read the cache")]
    Read(#[from] ReadError),
    #[error("failed to clear old frames")]
    Cleanup(#[from] CleanupError),
}

#[derive(thiserror::Error, Debug)]
pub enum WriteError {
    #[error("failed to write stats {path:?}")]
    Stats {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("failed to serialize state")]
    Serialization(#[from] Box<squishy_volumes_file_frame::Error>),
    #[error("failed to forward output frame to writing thread")]
    Sending,
    #[error("exceeding allowed disk space")]
    ExceedingSpace,
    #[error("internal error (panic) in the store thread: {message}, please report this bug")]
    StoreThreadPanicked { message: String },
}

#[derive(thiserror::Error, Debug)]
pub enum ReadError {
    #[error("some frames are missing from the sequence")]
    FrameSequenceBroken,
    #[error("frame is not computed yet")]
    FrameNotReady,
    #[error("failed to deserialize state")]
    Deserialization(#[from] Box<squishy_volumes_file_frame::Error>),
    #[error("failed to list cache directory {path:?}")]
    ListDirectory {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("failed to read size of frame {path:?}")]
    FrameSize {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum CleanupError {
    #[error(
        "failed to remove frame {frame} at {path:?}, it might still be open in another program (e.g. Blender displaying it)"
    )]
    RemoveFrame {
        frame: usize,
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("failed to discover frames to remove")]
    DiscoverFrames(#[source] ReadError),
    #[error("failed to read stats {path:?}")]
    ReadStats {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("failed to truncate stats {path:?}")]
    TruncateStats {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}
