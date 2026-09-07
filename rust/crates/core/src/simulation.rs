// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{num::NonZero, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value, to_value};
use squishy_volumes_cache::Cache;
use squishy_volumes_directory_lock::DirectoryLock;
use squishy_volumes_file_input::{InputHeader, InputRanges, InputReader};
use squishy_volumes_xpu::ReportInfo;
use tracing::{info, warn};

use crate::{
    Error, SimulationInputImpl,
    attributes::{available_attributes, fetch_flat_attribute_f32, fetch_flat_attribute_i32},
    compute_thread::{ComputeThread, ComputeThreadSettings},
    simulation_input_path,
};

pub struct SimulationImpl {
    input_header: InputHeader,
    input_ranges: InputRanges,

    cache: Arc<Cache>,
    compute_thread: Option<ComputeThread>,
}

impl SimulationImpl {
    pub fn new(
        SimulationInputImpl {
            directory_lock,
            input_writer,
            max_bytes_on_disk,
            current_frame,
            ..
        }: SimulationInputImpl,
    ) -> Result<Self, Error> {
        info!("Creating new simulation");
        if current_frame.is_some() {
            return Err(Error::LeftoverInputFrame);
        }

        info!("Finalizing input");
        input_writer.flush().map_err(Error::FinalizingInput)?;

        Self::load_with_lock(directory_lock, max_bytes_on_disk, true)
    }

    pub fn load(uuid: String, directory: PathBuf) -> Result<Self, Error> {
        info!("Loading old simulation");
        let directory_lock = DirectoryLock::new(directory.clone(), uuid)?;
        Self::load_with_lock(directory_lock, u64::MAX, false)
    }

    fn load_with_lock(
        directory_lock: DirectoryLock,
        max_bytes_on_disk: u64,
        clean_up: bool,
    ) -> Result<Self, Error> {
        let mut input_reader = InputReader::new(simulation_input_path(directory_lock.directory()))
            .map_err(Error::StartInputReading)?;
        let input_header = input_reader.read_header().map_err(Error::ReadHeader)?;
        let input_ranges = InputRanges::new(&input_header.objects);
        info!(?input_ranges);

        let cache = Arc::new(
            Cache::new(directory_lock, input_reader.size(), max_bytes_on_disk)
                .map_err(Error::CacheCreation)?,
        );

        if clean_up {
            cache.drop_frames(0).map_err(Error::CacheDropFrames)?;
        }

        Ok(Self {
            input_header,
            input_ranges,
            cache,
            compute_thread: None,
        })
    }
}

#[derive(serde::Serialize)]
struct PollInfo {
    current_bytes_on_disk: u64,
    progress: Vec<ReportInfo>,
}

impl SimulationImpl {
    pub fn input_header_impl(&self) -> Result<Value, Error> {
        to_value(&self.input_header).map_err(Error::EncodingInputHeader)
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

    pub fn available_attributes_impl(&self) -> Result<Vec<Value>, Error> {
        available_attributes(&self.input_header)
            .map(|attribute| serde_json::to_value(attribute).map_err(Error::EncodingAttribute))
            .collect()
    }

    pub fn fetch_flat_attribute_f32_impl(
        &self,
        frame: usize,
        attribute: Value,
    ) -> Result<Vec<f32>, Error> {
        Ok(fetch_flat_attribute_f32(
            &self.input_header,
            &self.input_ranges,
            &self
                .cache
                .fetch_frame(frame)
                .map_err(Error::CacheFetch)?
                .io_state,
            &from_value(attribute).map_err(Error::ParseAttribute)?,
        )?)
    }

    pub fn fetch_flat_attribute_i32_impl(
        &self,
        frame: usize,
        attribute: Value,
    ) -> Result<Vec<i32>, Error> {
        Ok(fetch_flat_attribute_i32(
            &self.input_header,
            &self.input_ranges,
            &self
                .cache
                .fetch_frame(frame)
                .map_err(Error::CacheFetch)?
                .io_state,
            &from_value(attribute).map_err(Error::ParseAttribute)?,
        )?)
    }

    pub fn current_bytes_on_disk_impl(&self) -> u64 {
        self.cache.current_bytes_on_disk()
    }

    pub fn stats_impl(&self, frame: usize) -> Result<Value, Error> {
        serde_json::to_value(
            self.cache
                .fetch_frame(frame)
                .map_err(Error::CacheFetch)?
                .stats
                .clone(),
        )
        .map_err(Error::EncodingStats)
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
