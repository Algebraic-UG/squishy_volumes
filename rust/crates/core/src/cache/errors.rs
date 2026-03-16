// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::io;

use thiserror::Error;

use crate::{directory_lock::DirectoryLockingError, state::attributes::AttributeError};

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Failed to lock down cache: {0}")]
    Lock(#[from] DirectoryLockingError),
    #[error("An error occured while writing to cache: {0}")]
    Writing(#[from] CacheWritingError),
    #[error("An error occured while reading the cache: {0}")]
    Reading(#[from] CacheReadingError),
    #[error("An error occured while clearing old frames: {0}")]
    Cleanup(#[from] CacheCleanupError),
}

#[derive(Error, Debug)]
pub enum CacheCleanupError {
    #[error("Unknown io error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum CacheWritingError {
    #[error("Failed to create output frame: {0}")]
    CreateFrame(io::Error),
    #[error("Failed to write output frame: {0}")]
    WriteFrame(io::Error),
    #[error("Failed to move output frame: {0}")]
    MoveFrame(io::Error),
    #[error("Failed to serialize state: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("Failed to forward output frame to writing thread")]
    Sending,
    #[error("Store thread is gone")]
    ThreadGone,
    #[error("Store thread stopped")]
    ThreadStopped,
    #[error("Exceeding allowed disk space")]
    ExceedingSpace,
    #[error("Unknown io error: {0}")]
    IoError(#[from] io::Error),
}

#[derive(Error, Debug)]
pub enum CacheReadingError {
    #[error("Some frames are missing from the sequence")]
    FrameSequenceBroken,
    #[error("Frame is not computed yet")]
    FrameNotReady,
    #[error("Failed to read frame: {0}")]
    ReadFrame(io::Error),
    #[error("Failed to deserialize state: {0}")]
    Deserialization(#[from] bincode::Error),
    #[error("Failed to read an attribute: {0}")]
    Attribute(#[from] AttributeError),
    #[error("Unknown io error: {0}")]
    IoError(#[from] io::Error),
}
