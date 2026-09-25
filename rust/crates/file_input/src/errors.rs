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
    #[error("Index offset mishap")]
    OffsetReading(#[from] InputOffsetReadingError),
    #[error("Unknown read/write error")]
    IoError(#[from] std::io::Error),
    #[error("Unknown bincode error")]
    BincodeError(#[from] bincode::Error),
    #[error("A simple check failed")]
    FileUtil(#[from] squishy_volumes_file_util::Error),
    #[error("Frame #{frame} verifcation failed")]
    FrameVerifcationError {
        frame: usize,
        #[source]
        error: FrameVerifcationError,
    },
    #[error("Too many different colliders.")]
    TooManyColliders,

    #[error("Input bulk data type mismatch: expected {expected} but found {found}")]
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },

    #[error("Input bulk data cast failed")]
    CastFailed(#[from] bytemuck::PodCastError),
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
        attribute: String,
        found: usize,
        expected: usize,
    },
    #[error("Object error")]
    ObjectError(#[from] ObjectError),
}

#[derive(Error, Debug)]
pub enum ObjectError {
    #[error("'{name}': Changed to/from Particles/Collider")]
    ObjectChangedType { name: String },
    #[error("'{name}': Was not declared in input header")]
    ObjectNotInHeader { name: String },
}
