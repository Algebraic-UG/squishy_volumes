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

use std::{
    fmt::Debug,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use bincode::deserialize_from;
use tracing::info;

use crate::Header;

use super::{FrameBulk, InputError, InputFrame, InputOffsetReadingError, magic_bytes};

pub struct InputReader {
    size: u64,
    reader: BufReader<File>,
    frame_offsets: Vec<u64>,
    index_offset: u64,
    header: Header,
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

    pub fn header(&self) -> &Header {
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

fn read_header<R: Read + Seek>(mut r: R) -> Result<Header, InputError> {
    r.seek(SeekFrom::Start(
        squishy_volumes_file_util::DATA_OFFSET.try_into().unwrap(),
    ))?;
    Ok(deserialize_from(&mut r)?)
}
