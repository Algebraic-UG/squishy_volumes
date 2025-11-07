// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    fs::File,
    io::{BufWriter, Seek, Write},
    path::Path,
};

use bincode::serialize_into;

use crate::input::common::{MAGIC_LEN, VERSION_LEN};

use super::{InputError, InputFrame, InputHeader, magic_bytes, version_bytes};

pub struct InputWriter {
    writer: BufWriter<File>,
    frame_offsets: Vec<u64>,
}

impl InputWriter {
    pub fn new<P: AsRef<Path>>(path: P, header: InputHeader) -> Result<Self, InputError> {
        let mut writer = BufWriter::new(File::create(path)?);
        assert!(writer.write(&magic_bytes())? == MAGIC_LEN);
        assert!(writer.write(&version_bytes())? == VERSION_LEN);
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

        let index_offset = writer.stream_position()?;
        serialize_into(&mut writer, &frame_offsets)?;

        assert!(writer.write(&index_offset.to_le_bytes())? == 8);
        writer.flush()?;

        Ok(())
    }
}
