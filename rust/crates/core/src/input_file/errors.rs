// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use thiserror::Error;

use crate::{elastic::EnergyError, mesh::MeshError};

#[derive(Error, Debug)]
pub enum InputError {
    #[error("The magic number didn't match, this is not a squishy volumes input file")]
    MagicMismatch,
    #[error("The version didn't match, this file needs version \"{0}\"")]
    VersionMismatch(String),
    #[error("Requested frame {requested} but there are only {available}")]
    FrameNotAvailable { requested: usize, available: usize },
    #[error("Index offset mishap: {0:?}")]
    OffsetReading(#[from] InputOffsetReadingError),
    #[error("Unknown read/write error")]
    IoError(#[from] std::io::Error),
    #[error("Unknown bincode error")]
    BincodeError(#[from] bincode::Error),
}

#[derive(Error, Debug)]
pub enum InputOffsetReadingError {
    #[error("Unknown read/write error")]
    IoError(#[from] std::io::Error),
    #[error("Unknown bincode error")]
    BincodeError(#[from] bincode::Error),
}

#[derive(Error, Debug)]
pub enum InputGenerationError {
    #[error("Dialation must be positive")]
    DilationError,
    #[error("Initial energy calculation failed")]
    Energy(#[from] EnergyError),
    #[error("Something is wrong with the mesh")]
    Mesh(#[from] MeshError),
    #[error("Sampling returned nothing")]
    NoSamples,
    #[error("The key for this bulk data was already present: {0}")]
    KeyAlreadyPresent(String),
}
