// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(thiserror::Error, Debug)]
pub enum CacheError {
    #[error("An error occured while writing to cache: {0}")]
    Writing(#[from] CacheWritingError),
    #[error("An error occured while reading the cache: {0}")]
    Reading(#[from] CacheReadingError),
    #[error("An error occured while clearing old frames: {0}")]
    Cleanup(#[from] CacheCleanupError),
    #[error("Something went really wrong and the store thread mutex is poisoned")]
    StoreThreadLockPoisoned,
    #[error("Directory lock error: {0}")]
    DirectoryLock(#[from] squishy_volumes_directory_lock::DirectoryLockingError),
}

#[derive(thiserror::Error, Debug)]
pub enum CacheWritingError {
    #[error("Failed to create output frame: {0}")]
    CreateFrame(std::io::Error),
    #[error("Failed to write output frame: {0}")]
    WriteFrame(std::io::Error),
    #[error("Failed to move output frame: {0}")]
    MoveFrame(std::io::Error),
    #[error("Failed to serialize state: {0}")]
    Serialization(#[from] squishy_volumes_file_frame::Error),
    #[error("Failed to forward output frame to writing thread")]
    Sending,
    #[error("Store thread is gone")]
    ThreadGone,
    #[error("Store thread stopped")]
    ThreadStopped,
    #[error("Exceeding allowed disk space")]
    ExceedingSpace,
    #[error("Unknown io error: {0}")]
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
    #[error("Failed to read frame: {0}")]
    ReadFrame(std::io::Error),
    #[error("Failed to deserialize state: {0}")]
    Deserialization(#[from] squishy_volumes_file_frame::Error),
    #[error("Unknown io error: {0}")]
    IoError(#[from] std::io::Error),
}
#[derive(thiserror::Error, Debug)]
pub enum CacheCleanupError {
    #[error("Unknown io error: {0}")]
    IoError(#[from] std::io::Error),
}
