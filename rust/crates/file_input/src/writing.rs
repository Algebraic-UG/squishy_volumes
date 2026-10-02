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
    io::{BufWriter, Seek, Write},
    path::Path,
};

use bincode::serialize_into;
use tracing::info;

use super::{FrameBulk, InputError, InputHeader, magic_bytes};

pub struct InputWriter {
    header: InputHeader,
    writer: BufWriter<File>,
    frame_offsets: Vec<u64>,
}

impl InputWriter {
    pub fn new<P: AsRef<Path> + Debug>(path: P, header: InputHeader) -> Result<Self, InputError> {
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
