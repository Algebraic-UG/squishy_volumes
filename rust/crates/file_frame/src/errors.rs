// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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
pub enum WriteErrorPoints {
    #[error("Failed to finish")]
    Finish(#[from] alembic_rust::Error),
    #[error("An attribute error occurred")]
    Attribute {
        name: String,
        #[source]
        error: alembic_rust::Error,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum ReadErrorPoints {
    #[error("Missing attrbute '{0}'")]
    AttributeMissing(String),
    #[error("An Alembic error occurred")]
    Alembic {
        name: String,
        #[source]
        error: alembic_rust::Error,
    },
}
