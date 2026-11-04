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
    fs::remove_file,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value};
use squishy_volumes_api::InputBulk;
use squishy_volumes_directory_lock::DirectoryLock;
use squishy_volumes_file_input::{FrameBulk, FrameBulkMeta, Header, InputHeader, InputWriter};
use squishy_volumes_util::AnimatedGlobals;

use crate::Error;

pub struct SimulationInputImpl {
    pub uuid: String,
    pub directory_lock: DirectoryLock,
    pub input_writer: InputWriter,
    pub max_bytes_on_disk: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FrameStart {
    pub animated_globals: AnimatedGlobals,
}

pub fn simulation_input_path<P: AsRef<Path>>(cache_dir: P) -> PathBuf {
    cache_dir.as_ref().join("simulation_input.bin")
}

#[allow(clippy::result_large_err)]
impl SimulationInputImpl {
    pub fn new(
        uuid: String,
        directory: PathBuf,
        input_header: InputHeader,
        max_bytes_on_disk: u64,
    ) -> Result<Self, Error> {
        let directory_lock = DirectoryLock::new(directory.clone())?;

        let input_writer =
            InputWriter::new(simulation_input_path(directory), Header::new(input_header))
                .map_err(Error::StartInputWriting)?;

        Ok(Self {
            uuid,
            directory_lock,
            input_writer,
            max_bytes_on_disk,
        })
    }

    pub fn clean_up(self) {
        drop(self.input_writer);
        if let Err(e) = remove_file(simulation_input_path(self.directory_lock.directory())) {
            tracing::error!("failed to clean up input file: {e:?}");
        }
    }

    pub fn start_frame_impl(&mut self, frame_start: Value) -> Result<(), Error> {
        let FrameStart { animated_globals } =
            from_value(frame_start).map_err(Error::ParsingFrameStart)?;

        let size = self
            .input_writer
            .start_frame(&animated_globals)
            .map_err(Error::StartFrame)?;

        self.check_vs_max_bytes(size)
    }

    pub fn record_input_impl(&mut self, meta: Value, bulk: InputBulk) -> Result<(), Error> {
        let meta = from_value::<FrameBulkMeta>(meta).map_err(Error::ParsingBulkMeta)?;

        let size = self
            .input_writer
            .record_bulk(&FrameBulk { meta, data: bulk })
            .map_err(Error::RecordFrame)?;

        self.check_vs_max_bytes(size)
    }

    fn check_vs_max_bytes(&self, size: u64) -> Result<(), Error> {
        if size > self.max_bytes_on_disk {
            Err(Error::DiskSpaceExceededWhileRecording(
                self.max_bytes_on_disk,
            ))
        } else {
            Ok(())
        }
    }
}
