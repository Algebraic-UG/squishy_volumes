// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{num::NonZero, path::PathBuf, sync::Arc};

use anyhow::{Result, ensure};
use serde_json::{Value, from_value, to_value};
use squishy_volumes_api::{ComputeSettings, Simulation, T, Task};
use tracing::{info, warn};

use crate::{
    SimulationInputImpl,
    cache::Cache,
    compute_thread::{ComputeThread, ComputeThreadSettings},
    directory_lock::DirectoryLock,
    input_file::{InputHeader, InputReader},
    input_interpolation::InputInterpolation,
    math::flat::Flat3,
    phase::PhaseInput,
    simulation_input_path,
    state::attributes::{Attribute, AttributeConst},
    stats::{ComputeStats, Stats},
};

pub struct SimulationImpl {
    directory_lock: DirectoryLock,

    input_reader: InputReader,
    input_header: InputHeader,

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
    ) -> Result<Self> {
        info!("Creating new simulation");
        ensure!(current_frame.is_none(), "Last frame wasn't written");

        input_writer.flush()?;

        Self::load_with_lock(directory_lock, max_bytes_on_disk)
    }

    pub fn load(uuid: String, directory: PathBuf) -> Result<Self> {
        info!("Loading old simulation");
        let directory_lock = DirectoryLock::new(directory.clone(), uuid)?;
        Self::load_with_lock(directory_lock, u64::MAX)
    }

    fn load_with_lock(directory_lock: DirectoryLock, max_bytes_on_disk: u64) -> Result<Self> {
        let mut input_reader = InputReader::new(simulation_input_path(directory_lock.directory()))?;
        let input_header = input_reader.read_header()?;
        let cache = Arc::new(Cache::load(
            directory_lock.directory().to_path_buf(),
            input_reader.size(),
            max_bytes_on_disk,
        )?);

        Ok(Self {
            directory_lock,
            input_reader,
            input_header,
            cache,
            compute_thread: None,
            cached_compute_stats: None,
        })
    }
}

impl Simulation for SimulationImpl {
    fn input_header(&self) -> Result<Value> {
        Ok(to_value(&self.input_header)?)
    }

    fn computing(&self) -> bool {
        self.compute_thread
            .as_ref()
            .is_some_and(ComputeThread::running)
    }

    fn poll(&mut self) -> Result<Option<Task>> {
        self.directory_lock.check()?;
        self.cache.check()?;
        self.compute_thread
            .as_mut()
            .map(ComputeThread::poll)
            .unwrap_or(Ok(Default::default()))
    }

    fn start_compute(
        &mut self,
        ComputeSettings {
            time_step,
            explicit,
            debug_mode,
            adaptive_time_steps,
            next_frame,
            number_of_frames,
            max_bytes_on_disk,
        }: ComputeSettings,
    ) -> Result<()> {
        info!("starting compute");
        self.cache.set_max_bytes_on_disk(max_bytes_on_disk);

        let Some(number_of_frames) = NonZero::new(number_of_frames) else {
            warn!("asked to compute 0 frames");
            return Ok(());
        };

        if next_frame >= number_of_frames.get() {
            warn!("no point in computing");
            return Ok(());
        }

        ensure!(
            next_frame <= self.available_frames(),
            "frame not computed yet"
        );

        self.pause_compute();

        info!("performing checks");
        info!("directory checks");
        self.directory_lock.check()?;
        info!("cache checks");
        self.cache.check()?;
        info!("drop checks");
        self.cache.drop_frames(next_frame)?;
        info!("input checks");
        let input_reader =
            InputReader::new(simulation_input_path(self.directory_lock.directory()))?;

        info!("starting thread");
        self.compute_thread = Some(ComputeThread::new(ComputeThreadSettings {
            consts: self.input_header.consts.clone(),
            input_reader,
            cache: self.cache.clone(),
            time_step,
            max_time_step: time_step,
            number_of_frames,
            next_frame,
            adaptive_time_steps,
            explicit,
            debug_mode,
        })?);
        Ok(())
    }

    fn pause_compute(&mut self) {
        self.cached_compute_stats = self
            .compute_thread
            .as_ref()
            .and_then(|compute_thread| compute_thread.stats.lock().unwrap().clone());
        self.compute_thread = None
    }

    fn available_frames(&self) -> usize {
        self.cache.available_frames()
    }

    fn available_attributes(&self, frame: usize) -> Result<Vec<Value>> {
        self.cache
            .available_attributes(frame)?
            .into_iter()
            .map(|attribute| Ok(to_value(attribute)?))
            .collect()
    }

    fn fetch_flat_attribute(&self, frame: usize, attribute: Value) -> Result<Vec<T>> {
        let attribute = from_value(attribute)?;
        match attribute {
            Attribute::Const(attribute) => Ok(match attribute {
                AttributeConst::GridNodeSize => vec![self.input_header.consts.grid_node_size],
                AttributeConst::FramesPerSecond => {
                    vec![self.input_header.consts.frames_per_second as T]
                }
                AttributeConst::SimulationScale => {
                    vec![self.input_header.consts.simulation_scale]
                }
                AttributeConst::DomainMin => self.input_header.consts.domain_min.flat().into(),
                AttributeConst::DomainMax => self.input_header.consts.domain_max.flat().into(),
            }),
            attribute => Ok(self.cache.fetch_flat_attribute(
                self.input_header.consts.grid_node_size,
                frame,
                attribute,
            )?),
        }
    }

    fn stats(&self) -> Result<Value> {
        Ok(to_value(Stats {
            loaded_state: self.cache.loaded_state_stats(),
            compute: self
                .compute_thread
                .as_ref()
                .and_then(|compute_thread| compute_thread.stats.lock().unwrap().clone())
                .or(self.cached_compute_stats.clone()),
            bytes_on_disk: self.cache.current_bytes_on_disk(),
        })?)
    }
}
