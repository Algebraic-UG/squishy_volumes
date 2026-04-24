// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{num::NonZero, path::PathBuf, sync::Arc};

use anyhow::{Result, bail, ensure};
use nalgebra::Vector4;
use serde_json::{Value, from_value, to_value};
use squishy_volumes_api::{ComputeSettings, Simulation, T, Task};
use squishy_volumes_gpu::{
    DownloadsToHost, GpuContext, GpuError, PipelinePart, PrepareGrid, gpu_grid_to_cpu_grid,
    prepare_grid, wgpu,
};
use squishy_volumes_util::Flat3 as _;
use tracing::{info, warn};

use crate::{
    SimulationInputImpl,
    cache::{Cache, clean_up_frames},
    compute_thread::{ComputeThread, ComputeThreadSettings},
    directory_lock::DirectoryLock,
    input_file::{InputHeader, InputReader},
    simulation_input_path,
    state::{
        State,
        attributes::{Attribute, AttributeConst},
    },
    stats::{ComputeStats, Stats},
};

pub struct SimulationImpl {
    directory_lock: DirectoryLock,

    input_header: InputHeader,

    cache: Arc<Cache>,
    compute_thread: Option<ComputeThread>,
    cached_compute_stats: Option<ComputeStats>,

    gpu_context: Result<GpuContext, GpuError>,
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
        clean_up_frames(directory_lock.directory(), 0)?;

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

        let cache = Arc::new(Cache::new(
            directory_lock.directory().to_path_buf(),
            input_reader.size(),
            max_bytes_on_disk,
        )?);

        let gpu_context = GpuContext::new(input_header.consts.max_num_particles);

        Ok(Self {
            directory_lock,
            input_header,
            cache,
            compute_thread: None,
            cached_compute_stats: None,
            gpu_context,
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
            gpu,
            explicit,
            adaptive_time_steps,
            next_frame,
            number_of_frames,
            max_bytes_on_disk,
        }: ComputeSettings,
    ) -> Result<()> {
        if gpu && let Err(e) = self.gpu_context.as_ref() {
            // TODO
            bail!(e.to_string());
        }

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
        let mut input_reader =
            InputReader::new(simulation_input_path(self.directory_lock.directory()))?;

        if gpu && let Ok(context) = self.gpu_context.as_mut() {
            context.setup_allocator(100000000, "main allocator", true)?;
            context.setup_indirect_allocator(1000, "indirect allocator", true)?;

            let cell_size = self.input_header.consts.scaled_grid_node_size() * 2.;
            let mut state = State::new(input_reader.read_header()?, input_reader.read_frame(0)?)?;
            let positions: Vec<Vector4<f32>> = state
                .particles
                .positions
                .iter()
                .map(|p| p.push(0.))
                .collect();

            let settings = prepare_grid::Settings {
                workgroup_size: 64.try_into().unwrap(),
                dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
                cell_size,
                bit_count: 2.try_into().unwrap(),
            };
            let input = prepare_grid::Input::new(context.device(), settings.clone(), &positions);
            let pipeline_part = PrepareGrid::new(context, settings);

            let mut encoder = context.device().create_command_encoder(&Default::default());

            let prepare_grid::Output {
                indirect_cells,
                cell_ids,
                cell_owns,
                ..
            } = pipeline_part.record(
                context,
                &mut (&mut encoder).into(),
                input,
                prepare_grid::Parameters,
            )?;

            let downloads = DownloadsToHost::new(context, [indirect_cells, cell_ids, cell_owns]);
            downloads.copy(&mut encoder);

            context.queue().submit([encoder.finish()]);

            let downloads = downloads.prep();

            context
                .device()
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();

            let [indirect_cells, cell_ids, cell_owns] = downloads.try_into().unwrap();

            let nodes = gpu_grid_to_cpu_grid(
                indirect_cells.to_vec()[0],
                &cell_ids.to_vec(),
                &cell_owns.to_vec(),
            );
            info!(num_nodes = nodes.len());
            for node in nodes {
                ensure!(state.grid_momentum.map.insert(node.xyz(), 0).is_none());
            }

            self.cache.store_frame(state)?;
            info!("byee");
            return Ok(());
        }

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
                AttributeConst::GridNodeSize => {
                    vec![self.input_header.consts.unscaled_grid_node_size()]
                }
                AttributeConst::FramesPerSecond => {
                    vec![self.input_header.consts.frames_per_second as T]
                }
                AttributeConst::SimulationScale => {
                    vec![self.input_header.consts.simulation_scale]
                }
                AttributeConst::DomainMin => self.input_header.consts.domain_min.flat().into(),
                AttributeConst::DomainMax => self.input_header.consts.domain_max.flat().into(),
            }),
            attribute => {
                Ok(self
                    .cache
                    .fetch_flat_attribute(&self.input_header.consts, frame, attribute)?)
            }
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
