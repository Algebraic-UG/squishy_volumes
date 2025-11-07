// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

// The input is a binary file that contains a mix of parameters and bulk geometry.
//
// The file is meant to be filled quickly, so there is minimal processing done on the bulk.
// At some point before the input is used in the simulation, additional processing must happen.
//
// The structure is simple, there are a few things that should remain stable across versions
// followed by a bunch of things that are completely handled by serde and are version dependent.
// (there might be migration paths later)
//
// =============================================================================
// Stable:
// =============================================================================
//
// 32 Magic bytes: binary of "Squishy Volumes Input File Magic"
// 64 Version Bytes: any version string like "10.1337.42-alpha" should fit this
//
// =============================================================================
// Unstable:
// =============================================================================
//
// InputHeader: contains everything that is known from the start of input recording
//
// InputFrame: Potentially bulky input from frame 0
// InputFrame: Potentially bulky input from frame 1
// InputFrame: Potentially bulky input from frame 2
// ...
//
// Index: contains all the frame offsets and is constructed in memory while recording
//
// 8 Index length bytes: so one can jump to the start of the index.

use std::{
    array::from_fn,
    fs::File,
    io::{BufWriter, Read, Seek, Write},
    iter::once,
    path::Path,
};

use bincode::serialize_into;
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod frame;
mod header;

pub use frame::InputFrame;
pub use header::InputHeader;

build_info::build_info!(fn build_info);

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

pub struct InputWriting {
    writer: BufWriter<File>,
    frame_offsets: Vec<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Frame {
    offset: u64,
    size: u64,
}
#[derive(Debug, Serialize, Deserialize)]
struct Index(Vec<Frame>);

impl InputWriting {
    pub fn new<P: AsRef<Path>>(path: P, header: InputHeader) -> Result<Self, InputError> {
        let mut writer = BufWriter::new(File::create(path)?);
        writer.write(&magic_bytes())?;
        writer.write(&version_bytes())?;
        serialize_into(&mut writer, &header)?;
        Ok(Self {
            writer,
            frame_offsets: Default::default(),
        })
    }

    pub fn record_frame(&mut self, frame: InputFrame) -> Result<(), InputError> {
        let current_offset = self.writer.stream_position()?;
        self.frame_offsets.push(current_offset);
        serialize_into(&mut self.writer, &frame)?;
        Ok(())
    }

    pub fn flush(self) -> Result<(), InputError> {
        let Self {
            mut writer,
            frame_offsets,
        } = self;
        let current_offset = writer.stream_position()?;
        let offsets = frame_offsets.into_iter();
        let index = Index(
            offsets
                .clone()
                .zip(offsets.skip(1).chain(once(current_offset)))
                .map(|(frame_start, frame_end)| {
                    assert!(frame_start <= frame_end);
                    Frame {
                        offset: frame_start,
                        size: frame_end - frame_start,
                    }
                })
                .collect(),
        );

        let current_offset = writer.stream_position()?;
        serialize_into(&mut writer, &index)?;
        writer.write(&current_offset.to_le_bytes())?;
        writer.flush()?;

        Ok(())
    }
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
