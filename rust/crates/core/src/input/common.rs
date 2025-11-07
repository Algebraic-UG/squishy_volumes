// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::array::from_fn;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InputError {
    #[error("The magic number didn't match, this is not a squishy volumes input file")]
    MagicMismatch,
    #[error("The version didn't match, this file needs version \"{0}\"")]
    VersionMismatch(String),
    #[error("Unknown read/write error")]
    IoError(#[from] std::io::Error),
    #[error("Unknown bincode error")]
    BincodeError(#[from] bincode::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Frame {
    pub offset: u64,
    pub size: u64,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Index(pub Vec<Frame>);

pub const MAGIC_LEN: usize = 32;
pub const VERSION_LEN: usize = 64;

pub fn magic_bytes() -> [u8; MAGIC_LEN] {
    const MAGIC: [char; MAGIC_LEN] = [
        'S', 'q', 'u', 'i', 's', 'h', 'y', ' ', //
        'V', 'o', 'l', 'u', 'm', 'e', 's', ' ', //
        'I', 'n', 'p', 'u', 't', ' ', //
        'F', 'i', 'l', 'e', ' ', //
        'M', 'a', 'g', 'i', 'c',
    ];
    from_fn(|i| MAGIC[i] as u8)
}

build_info::build_info!(fn build_info);

pub fn version_bytes() -> [u8; VERSION_LEN] {
    let version_string = build_info().crate_info.version.to_string();
    let bytes = version_string.as_bytes();
    assert!(bytes.len() <= VERSION_LEN, "Version string too long");
    from_fn(|i| if i < VERSION_LEN { bytes[i] } else { 0 })
}
