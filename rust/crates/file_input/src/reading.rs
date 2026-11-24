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
    path::{Path, PathBuf},
};

use bincode::deserialize_from;
use squishy_volumes_util::AnimatedGlobals;
use tracing::info;

use crate::Header;

use super::{Error, FileError, FrameBulk, InputFrame, magic_bytes};

pub struct InputReader {
    path: PathBuf,
    size: u64,
    reader: BufReader<File>,
    frame_offsets: Vec<u64>,
    index_offset: u64,
    header: Header,
}

impl InputReader {
    pub fn new<A: AsRef<Path> + Debug>(path: A) -> Result<Self, Error> {
        info!("Starting to read input from {path:?}");
        let path = path.as_ref();
        Self::open(path.to_path_buf()).map_err(|source| Error::File {
            path: path.to_path_buf(),
            source,
        })
    }

    fn open(path: PathBuf) -> Result<Self, FileError> {
        let file = File::open(&path).map_err(|source| FileError::Open { source })?;
        let size = file
            .metadata()
            .map_err(|source| FileError::ReadMetadata { source })?
            .len();
        let mut reader = BufReader::new(file);
        squishy_volumes_file_util::read_magic_and_version(magic_bytes, &mut reader)
            .map_err(|source| FileError::ReadMagicAndVersion { source })?;
        let index_offset = read_index_offset(&mut reader)?;
        let frame_offsets = read_frame_offsets(&mut reader, index_offset)?;
        let header = read_header(&mut reader)?;

        Ok(Self {
            path,
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

    pub fn read_frame(&mut self, frame: usize) -> Result<InputFrame, Error> {
        let Some(&offset) = self.frame_offsets.get(frame) else {
            return Err(Error::FrameNotAvailable {
                requested: frame,
                available: self.frame_offsets.len(),
            });
        };
        let end = self
            .frame_offsets
            .get(frame + 1)
            .cloned()
            .unwrap_or(self.index_offset);

        let (animated_globals, raw_bulk) = self
            .read_raw_frame(frame, offset, end)
            .map_err(|source| self.file_error(source))?;

        let bulk = raw_bulk
            .into_iter()
            .map(|bulk| {
                bulk.verify(&self.header)
                    .map_err(|source| Error::FrameVerification { frame, source })?;
                Ok(bulk.into())
            })
            .collect::<Result<_, Error>>()?;

        Ok(InputFrame {
            animated_globals,
            bulk,
        })
    }

    fn read_raw_frame(
        &mut self,
        frame: usize,
        offset: u64,
        end: u64,
    ) -> Result<(AnimatedGlobals, Vec<FrameBulk<'static>>), FileError> {
        let read_failed = |source: bincode::Error| FileError::ReadFrame { frame, source };
        let reader = &mut self.reader;

        reader
            .seek(SeekFrom::Start(offset))
            .map_err(|source| read_failed(source.into()))?;

        let animated_globals = deserialize_from(&mut *reader).map_err(read_failed)?;
        let mut bulk = Vec::new();
        while reader
            .stream_position()
            .map_err(|source| read_failed(source.into()))?
            < end
        {
            bulk.push(deserialize_from(&mut *reader).map_err(read_failed)?);
        }

        let pos = reader
            .stream_position()
            .map_err(|source| read_failed(source.into()))?;
        if pos != end {
            return Err(FileError::FrameMisalign { frame, end, pos });
        }

        Ok((animated_globals, bulk))
    }

    fn file_error(&self, source: FileError) -> Error {
        Error::File {
            path: self.path.clone(),
            source,
        }
    }
}

fn read_index_offset<R: Read + Seek>(mut r: R) -> Result<u64, FileError> {
    let mut bytes: [u8; 8] = [0; 8];
    r.seek(SeekFrom::End(-8))
        .and_then(|_| r.read_exact(&mut bytes))
        .map_err(|source| FileError::ReadIndexOffset { source })?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_frame_offsets<R: Read + Seek>(mut r: R, index_offset: u64) -> Result<Vec<u64>, FileError> {
    r.seek(SeekFrom::Start(index_offset))
        .map_err(bincode::Error::from)
        .and_then(|_| deserialize_from(&mut r))
        .map_err(|source| FileError::ReadOffsets { source })
}

fn read_header<R: Read + Seek>(mut r: R) -> Result<Header, FileError> {
    r.seek(SeekFrom::Start(
        squishy_volumes_file_util::DATA_OFFSET.try_into().unwrap(),
    ))
    .map_err(bincode::Error::from)
    .and_then(|_| deserialize_from(&mut r))
    .map_err(|source| FileError::ReadHeader { source })
}
