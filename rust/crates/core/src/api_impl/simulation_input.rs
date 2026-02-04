// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail, ensure};
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value};
use squishy_volumes_api::{SimulationInput, T};

use crate::{
    directory_lock::DirectoryLock,
    input_file::{InputFrame, InputHeader, InputWriter},
};

pub struct SimulationInputImpl {
    pub directory_lock: DirectoryLock,
    pub input_writer: InputWriter,
    pub max_bytes_on_disk: u64,
    pub current_frame: Option<InputFrame>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct FrameStart {
    gravity: Vector3<T>,
}

pub fn simulation_input_path<P: AsRef<Path>>(cache_dir: P) -> PathBuf {
    cache_dir.as_ref().join("simulation_input.bin")
}

impl SimulationInputImpl {
    pub fn new(
        uuid: String,
        directory: PathBuf,
        input_header: InputHeader,
        max_bytes_on_disk: u64,
    ) -> Result<Self> {
        let directory_lock = DirectoryLock::new(directory.clone(), uuid)?;

        let input_writer = InputWriter::new(simulation_input_path(directory), input_header)?;

        Ok(Self {
            directory_lock,
            input_writer,
            max_bytes_on_disk,
            current_frame: None,
        })
    }
}

impl SimulationInput for SimulationInputImpl {
    fn start_frame(&mut self, frame_start: Value) -> Result<()> {
        ensure!(self.current_frame.is_none(), "Not finished prior frame.");

        let FrameStart { gravity } = from_value(frame_start)?;

        self.current_frame = Some(InputFrame {
            gravity,
            bulk: Default::default(),
        });

        Ok(())
    }

    fn record_input(&mut self, meta: Value, bulk: squishy_volumes_api::InputBulk) -> Result<()> {
        let Some(current_frame) = self.current_frame.as_mut() else {
            bail!("No frame started.");
        };

        todo!()
    }

    fn finish_frame(&mut self) -> Result<()> {
        let Some(current_frame) = self.current_frame.take() else {
            bail!("No frame started.");
        };

        self.input_writer.record_frame(current_frame)?;

        ensure!(
            self.input_writer.size()? < self.max_bytes_on_disk,
            "Exceeding allowed disk space."
        );

        Ok(())
    }
}
