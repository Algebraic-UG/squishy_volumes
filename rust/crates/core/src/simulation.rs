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

use std::{num::NonZero, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value, to_value};
use squishy_volumes_cache::Cache;
use squishy_volumes_directory_lock::DirectoryLock;
use squishy_volumes_file_input::{Header, InputReader};
use squishy_volumes_xpu::ReportInfo;
use tracing::{info, warn};

use crate::{
    Error, SimulationInputImpl,
    compute_thread::{ComputeThread, ComputeThreadSettings},
    simulation_input_path,
};

pub struct SimulationImpl {
    header: Header,

    cache: Arc<Cache>,
    compute_thread: Option<ComputeThread>,
}

#[derive(serde::Serialize)]
struct PollInfo {
    current_bytes_on_disk: u64,
    progress: Vec<ReportInfo>,
}

#[allow(clippy::result_large_err)]
impl SimulationImpl {
    pub fn new(
        SimulationInputImpl {
            directory_lock,
            input_writer,
            max_bytes_on_disk,
            ..
        }: SimulationInputImpl,
    ) -> Result<Self, Error> {
        info!("Creating new simulation");

        info!("Finalizing input");
        let input_size_in_bytes = input_writer.flush().map_err(Error::FinalizingInput)?;
        info!(input_size_in_bytes);

        Self::load_with_lock(directory_lock, max_bytes_on_disk, true)
    }

    pub fn load(directory: PathBuf) -> Result<Self, Error> {
        info!("Loading old simulation");
        let directory_lock = DirectoryLock::new(directory)?;
        Self::load_with_lock(directory_lock, u64::MAX, false)
    }

    fn load_with_lock(
        directory_lock: DirectoryLock,
        max_bytes_on_disk: u64,
        clean_up: bool,
    ) -> Result<Self, Error> {
        let input_reader = InputReader::new(simulation_input_path(directory_lock.directory()))
            .map_err(Error::StartInputReading)?;
        let header = input_reader.header().clone();

        let cache = Arc::new(
            Cache::new(directory_lock, input_reader.size(), max_bytes_on_disk)
                .map_err(Error::CacheCreation)?,
        );

        if clean_up {
            cache.drop_frames(0).map_err(Error::CacheDropFrames)?;
        }

        Ok(Self {
            header,
            cache,
            compute_thread: None,
        })
    }

    pub fn input_header_impl(&self) -> Result<Value, Error> {
        to_value(&self.header).map_err(Error::EncodingInputHeader)
    }

    pub fn computing_impl(&self) -> bool {
        self.compute_thread
            .as_ref()
            .is_some_and(ComputeThread::running)
    }

    pub fn poll_impl(&mut self) -> Result<Value, Error> {
        self.cache.check().map_err(Error::CacheCheck)?;
        serde_json::to_value(PollInfo {
            current_bytes_on_disk: self.cache.current_bytes_on_disk(),
            progress: self
                .compute_thread
                .as_mut()
                .map(ComputeThread::poll)
                .transpose()?
                .unwrap_or(Default::default()),
        })
        .map_err(Error::EncodingReport)
    }

    pub fn start_compute_impl(&mut self, compute_settings: Value) -> Result<(), Error> {
        info!("starting compute");
        let ComputeSettings {
            time_step,
            gpu,
            adaptive_time_steps,
            next_frame,
            number_of_frames,
            max_bytes_on_disk,
        } = from_value(compute_settings).map_err(Error::ParsingComputeSettings)?;
        self.cache.set_max_bytes_on_disk(max_bytes_on_disk);

        let Some(number_of_frames) = NonZero::new(number_of_frames) else {
            warn!("asked to compute 0 frames");
            return Ok(());
        };

        self.pause_compute_impl();

        self.cache.check().map_err(Error::CacheCheck)?;
        self.cache
            .drop_frames(next_frame)
            .map_err(Error::CacheDropFrames)?;

        info!("starting thread");
        self.compute_thread = Some(ComputeThread::new(ComputeThreadSettings {
            cache: self.cache.clone(),
            max_time_step: time_step,
            number_of_frames,
            next_frame,
            adaptive_time_steps,
            gpu,
        })?);

        Ok(())
    }

    pub fn pause_compute_impl(&mut self) {
        self.compute_thread = None;
    }

    pub fn available_frames_impl(&self) -> usize {
        self.cache.available_frames()
    }

    pub fn current_bytes_on_disk_impl(&self) -> u64 {
        self.cache.current_bytes_on_disk()
    }
}

#[derive(Serialize, Deserialize)]
pub struct ComputeSettings {
    pub time_step: f32,
    pub gpu: Option<String>,
    pub adaptive_time_steps: bool,
    pub next_frame: usize,
    pub number_of_frames: usize,
    pub max_bytes_on_disk: u64,
}
