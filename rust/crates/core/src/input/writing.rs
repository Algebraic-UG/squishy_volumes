// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    fs::File,
    io::{BufWriter, Read, Seek, Write},
    iter::once,
    path::Path,
};

use bincode::serialize_into;

use super::{Frame, Index, InputError, InputFrame, InputHeader, magic_bytes, version_bytes};

pub struct InputWriter {
    writer: BufWriter<File>,
    frame_offsets: Vec<u64>,
}

impl InputWriter {
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
