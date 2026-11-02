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

use std::collections::BTreeSet;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error writing '{path}'")]
    Write {
        path: std::path::PathBuf,
        #[source]
        error: WriteError,
    },
    #[error("Error reading '{path}'")]
    Read {
        path: std::path::PathBuf,
        #[source]
        error: ReadError,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum WriteError {
    #[error("Failed to determine parent directory")]
    NoParent,
    #[error("Io error")]
    IoError(#[from] std::io::Error),
    #[error("An Alembic error occurred")]
    Alembic(#[from] alembic_rust::Error),
    #[error("Failed to serialize")]
    Serialize(#[from] bincode::Error),
    #[error("File utility error")]
    FileUtil(#[from] squishy_volumes_file_util::Error),
    #[error("Failed to flush")]
    Flush(#[from] std::io::IntoInnerError<std::io::BufWriter<std::fs::File>>),
    #[error("Points error for '{name}'")]
    Points {
        name: String,
        #[source]
        error: WriteErrorPoints,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum WriteErrorPoints {
    #[error("Failed to finish")]
    Finish(#[from] alembic_rust::Error),
    #[error("An attribute error occurred")]
    Attribute {
        name: String,
        #[source]
        error: WriteErrorPointsAttribute,
    },
    #[error("Attribute map incomplete, missing {0:?}'")]
    Incomplete(BTreeSet<squishy_volumes_util::attribute::Key>),
}

#[derive(thiserror::Error, Debug)]
pub enum WriteErrorPointsAttribute {
    #[error("Attribute is missing map")]
    Missing,
    #[error("An Alembic error occurred")]
    Alembic(#[from] alembic_rust::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum ReadError {
    #[error("Application key doesn't indicate Squishy Volumes: {0:?}")]
    ApplicationKey(Option<String>),
    #[error("The file is missing the binary blob")]
    MissingBlob,
    #[error("Io error")]
    IoError(#[from] std::io::Error),
    #[error("An Alembic error occurred")]
    Alembic(#[from] alembic_rust::Error),
    #[error("Failed to deserialize")]
    Deserialize(#[from] bincode::Error),
    #[error("File utility error")]
    FileUtil(#[from] squishy_volumes_file_util::Error),
    #[error("Points are missing '{0}'")]
    MissingPoints(String),
    #[error("Points error for '{name}'")]
    Points {
        name: String,
        #[source]
        error: ReadErrorPoints,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum ReadErrorPoints {
    #[error("Attribute error '{name}'")]
    Attribute {
        name: String,
        #[source]
        error: ReadErrorPointsAttribute,
    },
    #[error("Attribute map incomplete, missing {0:?}'")]
    Incomplete(BTreeSet<squishy_volumes_util::attribute::Key>),
}

#[derive(thiserror::Error, Debug)]
pub enum ReadErrorPointsAttribute {
    #[error("Attribute is missing in Alembic")]
    Missing,
    #[error("Attribute is duplicate")]
    Duplicate,
    #[error("An Alembic error occurred")]
    Alembic(#[from] alembic_rust::Error),
}
