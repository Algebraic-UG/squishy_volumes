// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

// The input is a binary file that contains a mix of parameters and bulk geometry.
// The structure is simple, there are a few things that should remain stable across versions
// and then a bunch of things that are completely handled by serde and are version dependent.
//
// Stable ===========
//
// 32 Magic bytes: binary of "Squishy Volumes Input File Magic"
// 64 Version Bytes: any version string like "10.1337.42-alpha" should fit this
//
// Unstable =========
//
// Header: this contains everything that is known from the start of input recoring
//
// Frame000: Potentially bulky input from frame 0
// Frame001: Potentially bulky input from frame 1
// Frame002: Potentially bulky input from frame 2
// ...
//
// Index: contains all the frame offsets and is constructed in memory while recording
//
// 8 Index length bytes: so one can jump to the start of the index.

use std::{
    array::from_fn,
    io::{Read, Write},
};

use thiserror::Error;

mod frame;
mod header;
mod index;

build_info::build_info!(fn build_info);

#[derive(Error, Debug)]
pub enum InputError {
    #[error("The magic number didn't match, this is not a squishy volumes input file")]
    MagicMismatch,
    #[error("The version didn't match, this file needs version \"{0}\"")]
    VersionMismatch(String),
    #[error("Unknown read/write error")]
    IoError(#[from] std::io::Error),
}

const MAGIC_LEN: usize = 32;
const VERSION_LEN: usize = 64;

fn magic_bytes() -> [u8; MAGIC_LEN] {
    const MAGIC: [char; MAGIC_LEN] = [
        'S', 'q', 'u', 'i', 's', 'h', 'y', ' ', //
        'V', 'o', 'l', 'u', 'm', 'e', 's', ' ', //
        'I', 'n', 'p', 'u', 't', ' ', //
        'F', 'i', 'l', 'e', ' ', //
        'M', 'a', 'g', 'i', 'c',
    ];
    from_fn(|i| MAGIC[i] as u8)
}

fn version_bytes() -> [u8; VERSION_LEN] {
    let version_string = build_info().crate_info.version.to_string();
    let bytes = version_string.as_bytes();
    assert!(bytes.len() <= VERSION_LEN, "Version string too long");
    from_fn(|i| if i < VERSION_LEN { bytes[i] } else { 0 })
}

fn write_magic<W: Write>(mut w: W) -> Result<(), InputError> {
    w.write(&magic_bytes())?;
    Ok(())
}

fn write_version<W: Write>(mut w: W) -> Result<(), InputError> {
    w.write(&version_bytes())?;
    Ok(())
}

fn read_magic<R: Read>(mut r: R) -> Result<(), InputError> {
    let mut bytes: [u8; MAGIC_LEN] = [0; MAGIC_LEN];
    r.read_exact(&mut bytes)?;
    if bytes != magic_bytes() {
        Err(InputError::MagicMismatch)
    } else {
        Ok(())
    }
}

fn read_version<R: Read>(mut r: R) -> Result<(), InputError> {
    let mut bytes: [u8; VERSION_LEN] = [0; VERSION_LEN];
    r.read_exact(&mut bytes)?;
    if bytes != version_bytes() {
        let version_string =
            String::from_utf8(bytes.iter().cloned().take_while(|b| *b != 0).collect())
                .unwrap_or_else(|_| format!("Failed to parse: {bytes:?}"));
        Err(InputError::VersionMismatch(version_string))
    } else {
        Ok(())
    }
}
