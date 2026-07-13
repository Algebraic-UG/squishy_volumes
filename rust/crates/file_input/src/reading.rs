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

use super::{InputError, InputFrame, InputHeader, InputOffsetReadingError, magic_bytes};

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
        squishy_volumes_file_util::read_magic_and_version(magic_bytes, &mut reader)?;
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

    pub fn is_empty(&self) -> bool {
        self.frame_offsets.is_empty()
    }

    pub fn len(&self) -> usize {
        self.frame_offsets.len()
    }

    pub fn read_header(&mut self) -> Result<InputHeader, InputError> {
        self.reader.seek(SeekFrom::Start(
            squishy_volumes_file_util::DATA_OFFSET.try_into().unwrap(),
        ))?;
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

fn read_frame_offsets<R: Read + Seek>(mut r: R) -> Result<Vec<u64>, InputOffsetReadingError> {
    let mut bytes: [u8; 8] = [0; 8];
    r.seek(SeekFrom::End(-8))?;
    r.read_exact(&mut bytes)?;
    let index_offset = u64::from_le_bytes(bytes);
    r.seek(SeekFrom::Start(index_offset))?;

    Ok(deserialize_from(&mut r)?)
}
