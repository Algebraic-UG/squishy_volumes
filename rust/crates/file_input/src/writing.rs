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
    io::{BufWriter, Seek, Write},
    path::{Path, PathBuf},
};

use bincode::serialize_into;
use tracing::info;

use super::{Error, FileError, FrameBulk, Header, magic_bytes};

pub struct InputWriter {
    path: PathBuf,
    header: Header,
    writer: BufWriter<File>,
    frame_offsets: Vec<u64>,
}

impl InputWriter {
    pub fn new<P: AsRef<Path> + Debug>(path: P, header: Header) -> Result<Self, Error> {
        info!("Start writing input to {path:?}");
        let path = path.as_ref();
        Self::create(path.to_path_buf(), header).map_err(|source| Error::File {
            path: path.to_path_buf(),
            source,
        })
    }

    fn create(path: PathBuf, header: Header) -> Result<Self, FileError> {
        let file = File::create(&path).map_err(|source| FileError::Create { source })?;
        let mut writer = BufWriter::new(file);
        squishy_volumes_file_util::write_magic_and_version(magic_bytes, &mut writer)
            .map_err(|source| FileError::WriteMagicAndVersion { source })?;
        serialize_into(&mut writer, &header).map_err(|source| FileError::WriteHeader { source })?;
        Ok(Self {
            path,
            header,
            writer,
            frame_offsets: Default::default(),
        })
    }

    pub fn start_frame(
        &mut self,
        animated_globals: &squishy_volumes_util::AnimatedGlobals,
    ) -> Result<u64, Error> {
        let frame = self.frame_offsets.len();
        self.write_frame_start(animated_globals)
            .map_err(|source| self.file_error(FileError::WriteFrame { frame, source }))
    }

    fn write_frame_start(
        &mut self,
        animated_globals: &squishy_volumes_util::AnimatedGlobals,
    ) -> Result<u64, bincode::Error> {
        let current_offset = stream_position(&mut self.writer)?;
        self.frame_offsets.push(current_offset);
        serialize_into(&mut self.writer, animated_globals)?;
        stream_position(&mut self.writer)
    }

    pub fn record_bulk(&mut self, bulk: &FrameBulk) -> Result<u64, Error> {
        if self.frame_offsets.is_empty() {
            return Err(Error::NoFrameStarted);
        }
        let frame = self.frame_offsets.len() - 1;
        bulk.verify(&self.header)
            .map_err(|source| Error::FrameVerification { frame, source })?;
        serialize_into(&mut self.writer, bulk)
            .and_then(|_| stream_position(&mut self.writer))
            .map_err(|source| self.file_error(FileError::WriteFrame { frame, source }))
    }

    pub fn flush(self) -> Result<u64, Error> {
        info!("Finish writing input");
        let Self {
            path,
            writer,
            frame_offsets,
            ..
        } = self;
        finish(writer, &frame_offsets).map_err(|source| Error::File { path, source })
    }

    fn file_error(&self, source: FileError) -> Error {
        Error::File {
            path: self.path.clone(),
            source,
        }
    }
}

fn stream_position(writer: &mut BufWriter<File>) -> Result<u64, bincode::Error> {
    writer.stream_position().map_err(bincode::Error::from)
}

fn finish(mut writer: BufWriter<File>, frame_offsets: &[u64]) -> Result<u64, FileError> {
    stream_position(&mut writer)
        .and_then(|index_offset| {
            serialize_into(&mut writer, frame_offsets)?;
            writer
                .write_all(&index_offset.to_le_bytes())
                .map_err(bincode::Error::from)
        })
        .map_err(|source| FileError::WriteOffsets { source })?;

    writer
        .flush()
        .and_then(|_| writer.stream_position())
        .map_err(|source| FileError::Flush { source })
}
