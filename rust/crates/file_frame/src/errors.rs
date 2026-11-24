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
    #[error("failed to write '{path}'")]
    Write {
        path: std::path::PathBuf,
        source: WriteError,
    },
    #[error("failed to read '{path}'")]
    Read {
        path: std::path::PathBuf,
        source: ReadError,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum WriteError {
    #[error("failed to determine parent directory")]
    NoParent,
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("alembic error")]
    Alembic(#[from] alembic_rust::Error),
    #[error("failed to serialize")]
    Serialize(#[from] bincode::Error),
    #[error("failed to write magic bytes and version")]
    FileUtil(#[from] squishy_volumes_file_util::Error),
    #[error("failed to flush")]
    Flush(#[from] std::io::IntoInnerError<std::io::BufWriter<std::fs::File>>),
    #[error("failed to write points '{name}'")]
    Points {
        name: String,
        source: WritePointsError,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum WritePointsError {
    #[error("failed to finish")]
    Finish(#[from] alembic_rust::Error),
    #[error("failed to write attribute '{name}'")]
    Attribute {
        name: String,
        source: WriteAttributeError,
    },
    #[error("attribute map incomplete, missing {0:?}")]
    Incomplete(BTreeSet<squishy_volumes_util::attribute::Key>),
}

#[derive(thiserror::Error, Debug)]
pub enum WriteAttributeError {
    #[error("attribute is missing in the map")]
    Missing,
    #[error("alembic error")]
    Alembic(#[from] alembic_rust::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum ReadError {
    #[error("application key doesn't indicate Squishy Volumes: {0:?}")]
    ApplicationKey(Option<String>),
    #[error("the file is missing the binary blob")]
    MissingBlob,
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("alembic error")]
    Alembic(#[from] alembic_rust::Error),
    #[error("failed to deserialize")]
    Deserialize(#[from] bincode::Error),
    #[error("failed to check magic bytes and version")]
    FileUtil(#[from] squishy_volumes_file_util::Error),
    #[error("points '{0}' are missing")]
    MissingPoints(String),
    #[error("failed to read points '{name}'")]
    Points {
        name: String,
        source: ReadPointsError,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum ReadPointsError {
    #[error("failed to read attribute '{name}'")]
    Attribute {
        name: String,
        source: ReadAttributeError,
    },
    #[error("attribute map incomplete, missing {0:?}")]
    Incomplete(BTreeSet<squishy_volumes_util::attribute::Key>),
}

#[derive(thiserror::Error, Debug)]
pub enum ReadAttributeError {
    #[error("attribute is missing in the alembic file")]
    Missing,
    #[error("attribute is duplicate")]
    Duplicate,
    #[error("alembic error")]
    Alembic(#[from] alembic_rust::Error),
}
