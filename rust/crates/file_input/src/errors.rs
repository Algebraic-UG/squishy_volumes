// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use thiserror::Error;

use crate::BulkAttribute;

#[derive(Error, Debug)]
pub enum InputError {
    #[error("Can't record input bulk if no frame is started")]
    NoFrameStarted,
    #[error("Expected frame to go to {end} but read to {pos}")]
    FrameMisalign { end: u64, pos: u64 },
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
    #[error("Object '{name}' error")]
    ObjectError {
        name: String,
        #[source]
        error: ObjectError,
    },
}

impl FrameVerifcationError {
    pub fn attach_frame(self, frame: usize) -> InputError {
        InputError::FrameVerifcationError { frame, error: self }
    }
}

#[derive(Error, Debug)]
pub enum ObjectError {
    #[error("This object changed type to/from Particles/Collider")]
    ObjectChangedType,
    #[error("This object was not declared in input header")]
    ObjectNotInHeader,
    #[error("Attribute '{attr:?}' error")]
    AttributeError {
        attr: BulkAttribute,
        #[source]
        error: AttributeError,
    },
}

impl ObjectError {
    pub fn attach_name(self, name: String) -> FrameVerifcationError {
        FrameVerifcationError::ObjectError { name, error: self }
    }
}

#[derive(Error, Debug)]
pub enum AttributeError {
    #[error("Atttribute data type mismatch: expected {expected} but found {found}")]
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    #[error("Attribute data cast failed")]
    CastFailed(#[from] bytemuck::PodCastError),
    #[error("Attribute Lengh mismatch, expected {expected} but found {found}")]
    LengthMismatch { found: usize, expected: usize },
}

impl AttributeError {
    pub fn attach_attr(self, attr: BulkAttribute) -> ObjectError {
        ObjectError::AttributeError { attr, error: self }
    }
}
