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

use crate::BulkAttribute;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("can't record input bulk if no frame is started")]
    NoFrameStarted,
    #[error("expected frame to go to {end} but read to {pos}")]
    FrameMisalign { end: u64, pos: u64 },
    #[error("requested frame {requested} but there are only {available}")]
    FrameNotAvailable { requested: usize, available: usize },
    #[error("failed to read the frame offsets")]
    OffsetReading(#[from] InputOffsetReadingError),
    #[error("unknown read/write error")]
    Io(#[from] std::io::Error),
    #[error("unknown bincode error")]
    Bincode(#[from] bincode::Error),
    #[error("failed to check magic bytes and version")]
    FileUtil(#[from] squishy_volumes_file_util::Error),
    #[error("frame #{frame} verification failed")]
    FrameVerification {
        frame: usize,
        source: FrameVerificationError,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum InputOffsetReadingError {
    #[error("Unknown read/write error")]
    IoError(#[from] std::io::Error),
    #[error("Unknown bincode error")]
    BincodeError(#[from] bincode::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum FrameVerificationError {
    #[error("failed to verify object '{name}'")]
    Object { name: String, source: ObjectError },
}

impl FrameVerificationError {
    pub fn attach_frame(self, frame: usize) -> Error {
        Error::FrameVerification {
            frame,
            source: self,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ObjectError {
    #[error("object changed type to/from particles/collider")]
    ChangedType,
    #[error("object was not declared in the input header")]
    NotInHeader,
    #[error("failed to verify attribute '{attr:?}'")]
    Attribute {
        attr: BulkAttribute,
        source: BulkAttributeError,
    },
}

impl ObjectError {
    pub fn attach_name(self, name: String) -> FrameVerificationError {
        FrameVerificationError::Object { name, source: self }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BulkAttributeError {
    #[error("attribute data type mismatch: expected {expected} but found {found}")]
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    #[error("failed to cast attribute data")]
    Cast(#[from] bytemuck::PodCastError),
    #[error("attribute length mismatch: expected {expected} but found {found}")]
    LengthMismatch { found: usize, expected: usize },
}

impl BulkAttributeError {
    pub fn attach_attr(self, attr: BulkAttribute) -> ObjectError {
        ObjectError::Attribute { attr, source: self }
    }
}
