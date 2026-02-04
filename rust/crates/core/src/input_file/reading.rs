// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    fmt::Debug,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use bincode::deserialize_from;
use tracing::info;

use super::{
    InputError, InputFrame, InputHeader, InputOffsetReadingError, MAGIC_LEN, VERSION_LEN,
    common::HEADER_OFFSET, magic_bytes, version_bytes,
};

pub struct InputReader {
    size: u64,
    reader: BufReader<File>,
    frame_offsets: Vec<u64>,
}

impl InputReader {
    pub fn new<A: AsRef<Path> + Debug>(path: A) -> Result<Self, InputError> {
        info!("Starting to read input from {path:?}");
        let file = File::open(path)?;
        let size = file.metadata()?.len();
        let mut reader = BufReader::new(file);
        read_magic(&mut reader)?;
        read_version(&mut reader)?;
        let frame_offsets = read_frame_offsets(&mut reader)?;
        Ok(Self {
            size,
            reader,
            frame_offsets,
        })
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn read_header(&mut self) -> Result<InputHeader, InputError> {
        self.reader
            .seek(SeekFrom::Start(HEADER_OFFSET.try_into().unwrap()))?;
        Ok(deserialize_from(&mut self.reader)?)
    }

    pub fn read_frame(&mut self, frame: usize) -> Result<InputFrame, InputError> {
        let Some(offset) = self.frame_offsets.get(frame) else {
            return Err(InputError::FrameNotAvailable {
                requested: frame,
                available: self.frame_offsets.len(),
            });
        };

        self.reader.seek(SeekFrom::Start(*offset))?;
        Ok(deserialize_from(&mut self.reader)?)
    }
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

fn read_frame_offsets<R: Read + Seek>(mut r: R) -> Result<Vec<u64>, InputOffsetReadingError> {
    let mut bytes: [u8; 8] = [0; 8];
    r.seek(SeekFrom::End(-8))?;
    r.read_exact(&mut bytes)?;
    let index_offset = u64::from_le_bytes(bytes);
    r.seek(SeekFrom::Start(index_offset))?;

    Ok(deserialize_from(&mut r)?)
}
