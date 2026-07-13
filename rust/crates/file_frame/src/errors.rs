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
    #[error("Failed to create '{temp}': {error}")]
    Create {
        temp: std::path::PathBuf,
        error: std::io::Error,
    },
    #[error("Failed to open '{path}': {error}")]
    Open {
        path: std::path::PathBuf,
        error: std::io::Error,
    },
    #[error("Failed to flush '{temp}': {error}")]
    Write {
        temp: std::path::PathBuf,
        error: std::io::IntoInnerError<std::io::BufWriter<std::fs::File>>,
    },
    #[error("Failed to read metadata of '{temp}': {error}")]
    Metadata {
        temp: std::path::PathBuf,
        error: std::io::Error,
    },
    #[error("Failed to move '{temp}' to '{path}': {error}")]
    Move {
        temp: std::path::PathBuf,
        path: std::path::PathBuf,
        error: std::io::Error,
    },
    #[error("Failed to serialize state: {0}")]
    Serialize(bincode::Error),
    #[error("Failed to read '{path}': {error}")]
    Read {
        path: std::path::PathBuf,
        error: std::io::Error,
    },
    #[error("Failed to serialize state: {0}")]
    Deserialize(bincode::Error),
    #[error("A simple check failed: {0}")]
    FileUtil(#[from] squishy_volumes_file_util::Error),
}
