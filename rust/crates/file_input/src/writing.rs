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
    path::Path,
};

use bincode::serialize_into;
use tracing::info;

use super::{FrameBulk, Header, InputError, magic_bytes};

pub struct InputWriter {
    header: Header,
    writer: BufWriter<File>,
    frame_offsets: Vec<u64>,
}

impl InputWriter {
    pub fn new<P: AsRef<Path> + Debug>(path: P, header: Header) -> Result<Self, InputError> {
        info!("Start writing input to {path:?}");
        let mut writer = BufWriter::new(File::create(path)?);
        squishy_volumes_file_util::write_magic_and_version(magic_bytes, &mut writer)?;
        serialize_into(&mut writer, &header)?;
        Ok(Self {
            header,
            writer,
            frame_offsets: Default::default(),
        })
    }

    pub fn start_frame(
        &mut self,
        animated_globals: &squishy_volumes_util::AnimatedGlobals,
    ) -> Result<u64, InputError> {
        let current_offset = self.writer.stream_position()?;
        self.frame_offsets.push(current_offset);
        serialize_into(&mut self.writer, animated_globals)?;

        Ok(self.writer.stream_position()?)
    }

    pub fn record_bulk(&mut self, bulk: &FrameBulk) -> Result<u64, InputError> {
        if self.frame_offsets.is_empty() {
            return Err(InputError::NoFrameStarted);
        }
        let frame = self.frame_offsets.len() - 1;
        bulk.verify(&self.header)
            .map_err(|error| InputError::FrameVerifcationError { frame, error })?;
        serialize_into(&mut self.writer, bulk)?;

        Ok(self.writer.stream_position()?)
    }

    pub fn flush(self) -> Result<u64, InputError> {
        info!("Finish writing input");
        let Self {
            mut writer,
            frame_offsets,
            ..
        } = self;

        let index_offset = writer.stream_position()?;
        serialize_into(&mut writer, &frame_offsets)?;

        writer.write_all(&index_offset.to_le_bytes())?;
        writer.flush()?;

        Ok(writer.stream_position()?)
    }
}
