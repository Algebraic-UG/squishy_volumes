// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{collections::BTreeMap, num::NonZero, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value, to_value};
use squishy_volumes_cache::Cache;
use squishy_volumes_directory_lock::DirectoryLock;
use squishy_volumes_file_input::{InputHeader, InputObject, InputRanges, InputReader};
use tracing::{info, warn};

use crate::{
    Error, SimulationInputImpl,
    attributes::{available_attributes, fetch_flat_attribute_f32, fetch_flat_attribute_i32},
    compute_thread::{ComputeThread, ComputeThreadSettings},
    simulation_input_path,
    stats::{ComputeStats, StateStats, Stats},
};

pub struct SimulationImpl {
    input_header: InputHeader,
    input_ranges: InputRanges,

    cache: Arc<Cache>,
    compute_thread: Option<ComputeThread>,
    cached_compute_stats: Option<ComputeStats>,
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
            cached_compute_stats: None,
        })
    }
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
        serde_json::to_value(
            self.compute_thread
                .as_mut()
                .map(ComputeThread::poll)
                .transpose()?
                .unwrap_or(Default::default()),
        )
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

        self.pause_compute_impl()?;

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

    pub fn pause_compute_impl(&mut self) -> Result<(), Error> {
        self.cached_compute_stats = None;
        if let Some(compute_thread) = self.compute_thread.take() {
            self.cached_compute_stats = compute_thread.stats()?;
        }
        Ok(())
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
            &*self.cache.fetch_frame(frame).map_err(Error::CacheFetch)?,
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
            &*self.cache.fetch_frame(frame).map_err(Error::CacheFetch)?,
            &from_value(attribute).map_err(Error::ParseAttribute)?,
        )?)
    }

    pub fn stats_impl(&self) -> Result<Value, Error> {
        let state = {
            let mut total_particle_count = 0;
            let per_object_count: BTreeMap<String, usize> = self
                .input_header
                .objects
                .iter()
                .filter_map(|(name, object)| {
                    if let InputObject::Particles { num_particles } = object {
                        total_particle_count += num_particles;
                        Some((name.clone(), *num_particles))
                    } else {
                        None
                    }
                })
                .collect();
            let grid_node_count = self
                .cache
                .grid_node_count()
                .map_err(Error::CacheNodeCount)?;

            StateStats {
                total_particle_count,
                per_object_count,
                grid_node_count,
            }
        };

        let compute = self
            .compute_thread
            .as_ref()
            .and_then(|compute_thread| compute_thread.stats().transpose())
            .transpose()?
            .or(self.cached_compute_stats.clone());

        let bytes_on_disk = self.cache.current_bytes_on_disk();

        to_value(Stats {
            state,
            compute,
            bytes_on_disk,
        })
        .map_err(Error::EncodingStats)
    }
}

#[derive(Serialize, Deserialize)]
pub struct ComputeSettings {
    pub time_step: f32,
    pub gpu: bool,
    pub adaptive_time_steps: bool,
    pub next_frame: usize,
    pub number_of_frames: usize,
    pub max_bytes_on_disk: u64,
}
