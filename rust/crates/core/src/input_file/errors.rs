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
