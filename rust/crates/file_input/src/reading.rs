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

use super::{FrameBulk, InputError, InputFrame, InputHeader, InputOffsetReadingError, magic_bytes};

pub struct InputReader {
    size: u64,
    reader: BufReader<File>,
    frame_offsets: Vec<u64>,
    index_offset: u64,
    header: InputHeader,
}

impl InputReader {
    pub fn new<A: AsRef<Path> + Debug>(path: A) -> Result<Self, InputError> {
        info!("Starting to read input from {path:?}");
        let file = File::open(path)?;
        let size = file.metadata()?.len();
        let mut reader = BufReader::new(file);
        squishy_volumes_file_util::read_magic_and_version(magic_bytes, &mut reader)?;
        let index_offset = read_index_offset(&mut reader)?;
        let frame_offsets = read_frame_offsets(&mut reader, index_offset)?;
        let header = read_header(&mut reader)?;

        Ok(Self {
            size,
            reader,
            frame_offsets,
            index_offset,
            header,
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

    pub fn header(&self) -> &InputHeader {
        &self.header
    }

    pub fn read_frame(&mut self, frame: usize) -> Result<InputFrame, InputError> {
        let Some(offset) = self.frame_offsets.get(frame) else {
            return Err(InputError::FrameNotAvailable {
                requested: frame,
                available: self.frame_offsets.len(),
            });
        };
        let end = self
            .frame_offsets
            .get(frame + 1)
            .cloned()
            .unwrap_or(self.index_offset);

        self.reader.seek(SeekFrom::Start(*offset))?;

        let animated_globals = deserialize_from(&mut self.reader)?;
        let mut bulk = Vec::new();
        while self.reader.stream_position()? < end {
            let tmp: FrameBulk = deserialize_from(&mut self.reader)?;
            tmp.verify(&self.header)
                .map_err(|error| InputError::FrameVerifcationError { frame, error })?;
            bulk.push(tmp.into());
        }

        let pos = self.reader.stream_position()?;
        if pos != end {
            return Err(InputError::FrameMisalign { end, pos });
        }

        Ok(InputFrame {
            animated_globals,
            bulk,
        })
    }
}

fn read_index_offset<R: Read + Seek>(mut r: R) -> Result<u64, InputOffsetReadingError> {
    let mut bytes: [u8; 8] = [0; 8];
    r.seek(SeekFrom::End(-8))?;
    r.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_frame_offsets<R: Read + Seek>(
    mut r: R,
    index_offset: u64,
) -> Result<Vec<u64>, InputOffsetReadingError> {
    r.seek(SeekFrom::Start(index_offset))?;
    Ok(deserialize_from(&mut r)?)
}

fn read_header<R: Read + Seek>(mut r: R) -> Result<InputHeader, InputError> {
    r.seek(SeekFrom::Start(
        squishy_volumes_file_util::DATA_OFFSET.try_into().unwrap(),
    ))?;
    Ok(deserialize_from(&mut r)?)
}
