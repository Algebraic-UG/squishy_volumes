// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum InputError {
    #[error("Requested frame {requested} but there are only {available}")]
    FrameNotAvailable { requested: usize, available: usize },
    #[error("Index offset mishap: {0:?}")]
    OffsetReading(#[from] InputOffsetReadingError),
    #[error("Unknown read/write error")]
    IoError(#[from] std::io::Error),
    #[error("Unknown bincode error")]
    BincodeError(#[from] bincode::Error),
    #[error("A simple check failed: {0}")]
    FileUtil(#[from] squishy_volumes_file_util::Error),
    #[error("Frame #{frame} verifcation failed: {error}")]
    FrameVerifcationError {
        frame: usize,
        error: FrameVerifcationError,
    },
    #[error("Too many different colliders.")]
    TooManyColliders,
}

#[derive(Error, Debug)]
pub enum InputOffsetReadingError {
    #[error("Unknown read/write error")]
    IoError(#[from] std::io::Error),
    #[error("Unknown bincode error")]
    BincodeError(#[from] bincode::Error),
}

#[derive(Error, Debug)]
pub enum FrameVerifcationError {
    #[error(
        "'{name}': Recorded attribute '{attribute}' has length {found} but expected {expected}"
    )]
    LengthMismatch {
        name: String,
        attribute: &'static str,
        found: usize,
        expected: usize,
    },
    #[error(
        "'{0}': Missing collider input, note that collider must be present in all input frames"
    )]
    ColliderInputMissing(String),
    #[error("Object error: {0}")]
    ObjectError(#[from] ObjectError),
}

#[derive(Error, Debug)]
pub enum ObjectError {
    #[error("'{name}': Changed to/from Particles/Collider")]
    ObjectChangedType { name: String },
    #[error("'{name}': Was not declared in input header")]
    ObjectNotInHeader { name: String },
}
