// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to determine directory of '{0}'")]
    NoParent(std::path::PathBuf),
    #[error("Failed to create '{temp}'")]
    Create {
        temp: std::path::PathBuf,
        #[source]
        error: std::io::Error,
    },
    #[error("Failed to open '{path}'")]
    Open {
        path: std::path::PathBuf,
        #[source]
        error: std::io::Error,
    },
    #[error("Failed to flush '{temp}'")]
    Write {
        temp: std::path::PathBuf,
        #[source]
        error: std::io::IntoInnerError<std::io::BufWriter<std::fs::File>>,
    },
    #[error("Failed to read metadata of '{temp}'")]
    Metadata {
        temp: std::path::PathBuf,
        #[source]
        error: std::io::Error,
    },
    #[error("Failed to move '{temp}' to '{path}'")]
    Move {
        temp: std::path::PathBuf,
        path: std::path::PathBuf,
        #[source]
        error: std::io::Error,
    },
    #[error("Failed to serialize state")]
    Serialize(#[source] bincode::Error),
    #[error("Failed to read '{path}'")]
    Read {
        path: std::path::PathBuf,
        #[source]
        error: std::io::Error,
    },
    #[error("Failed to serialize state")]
    Deserialize(#[source] bincode::Error),
    #[error("A simple check failed")]
    FileUtil(#[from] squishy_volumes_file_util::Error),
}
