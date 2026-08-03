// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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
    #[error("Directory lock error")]
    DirectoryLock(#[from] squishy_volumes_directory_lock::DirectoryLockingError),
}

#[derive(thiserror::Error, Debug)]
pub enum CacheWritingError {
    #[error("Failed to create output frame")]
    CreateFrame(#[source] std::io::Error),
    #[error("Failed to write output frame")]
    WriteFrame(#[source] std::io::Error),
    #[error("Failed to move output frame")]
    MoveFrame(#[source] std::io::Error),
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
    #[error("Unknown io error")]
    IoError(#[from] std::io::Error),
}
