// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    collections::VecDeque,
    num::NonZero,
    sync::{Arc, Mutex},
    thread::{JoinHandle, spawn},
    time::Instant,
};

use squishy_volumes_cache::Cache;
use squishy_volumes_cpu::{CpuRunParameters, CpuState};
use squishy_volumes_file_input::InputReader;
use squishy_volumes_gpu::{GpuRunParameters, GpuState};
use squishy_volumes_util::panic_payload_to_string;
use squishy_volumes_xpu::{FrameInput, Harness, ReportInfo};
use tracing::info;

#[cfg(feature = "profile")]
use squishy_volumes_util::coarse_prof;

use crate::{
    Error, initialization::initialize_io_state, simulation_input_path, stats::ComputeStats,
};

pub struct ComputeThread {
    stats: Arc<Mutex<Option<ComputeStats>>>,

    harness: Harness,
    thread: Option<JoinHandle<Result<(), Error>>>,
}

pub struct ComputeThreadSettings {
    pub cache: Arc<Cache>,

    pub max_time_step: f32,

    pub number_of_frames: NonZero<usize>,
    pub next_frame: usize,

    pub gpu: Option<String>,
    pub adaptive_time_steps: bool,
}

impl ComputeThread {
    pub fn new(
        ComputeThreadSettings {
            cache,
            max_time_step,
            number_of_frames,
            mut next_frame,
            adaptive_time_steps,
            gpu,
        }: ComputeThreadSettings,
    ) -> Result<Self, Error> {
        info!("starting compute thread");

        let mut input_reader = InputReader::new(simulation_input_path(cache.directory()))
            .map_err(Error::StartInputReading)?;
        let consts = input_reader
            .read_header()
            .map_err(Error::ReadHeader)?
            .consts;

        let stats = Arc::new(Mutex::new(None));
        let harness = Harness::new("Simulating Frames".to_string(), number_of_frames);
        harness.step_to(next_frame)?;

        let thread = {
            let stats = stats.clone();
            let harness = harness.clone();
            Some(spawn(move || -> Result<(), Error> {
                info!("compute thread started");
                let io_state = if next_frame == 0 {
                    info!("creating initial state");
                    let io_state = initialize_io_state(&harness, &mut input_reader)?;
                    cache
                        .store_frame(io_state.clone())
                        .map_err(Error::StoreError)?;
                    next_frame += 1;
                    harness.step()?;
                    io_state
                } else {
                    info!("loading checkpoint");
                    cache
                        .fetch_frame(next_frame - 1)
                        .map_err(Error::CacheFetch)?
                        .clone()
                };
                harness.check()?;

                let mut frame_input = FrameInput::new(input_reader, next_frame - 1)?;

                #[allow(clippy::large_enum_variant)]
                enum ComputeState {
                    Cpu(CpuState),
                    Gpu(GpuState),
                }

                let mut compute_state = if let Some(gpu) = gpu {
                    ComputeState::Gpu(GpuState::from_io_state(
                        gpu,
                        &harness,
                        &frame_input,
                        max_time_step,
                        io_state,
                        Some(cache.directory().join("gpu_profile.csv")),
                    )?)
                } else {
                    ComputeState::Cpu(CpuState::from_io_state(io_state)?)
                };

                #[cfg(feature = "profile")]
                {
                    coarse_prof::reset();
                    info!("profile reset");
                }

                let mut frame_times = VecDeque::new();
                while next_frame < number_of_frames.get() {
                    harness.check()?;

                    let start_compute_frame = Instant::now();

                    frame_input.load(next_frame - 1)?;

                    let target_time = next_frame as f64 / consts.frames_per_second as f64;

                    let result: Result<(), Error>;
                    let io_state = match &mut compute_state {
                        ComputeState::Cpu(cpu_state) => {
                            let (io_state, cpu_result) = cpu_state.produce_next_state(
                                &harness,
                                &frame_input,
                                CpuRunParameters {
                                    target_time,
                                    max_time_step,
                                    adaptive_time_steps,
                                    store_grid: true,
                                },
                            )?;
                            result = cpu_result.map_err(Error::CpuCompute);
                            io_state
                        }
                        ComputeState::Gpu(gpu_state) => {
                            let (io_state, gpu_result) = gpu_state.produce_next_state(
                                &harness,
                                &mut frame_input,
                                GpuRunParameters {
                                    target_time,
                                    store_grid: true,
                                },
                            )?;
                            result = gpu_result.map_err(Error::GpuError);
                            io_state
                        }
                    };

                    // store state even if error occured
                    cache.store_frame(io_state).map_err(Error::StoreError)?;

                    // now check for errors
                    result?;

                    info!("computed frame {} of {}", next_frame, number_of_frames);

                    next_frame += 1;
                    harness.step()?;

                    let last_frame_time_sec = start_compute_frame.elapsed().as_secs_f32();
                    let remaining_frames = number_of_frames.get() - next_frame;

                    frame_times.push_back(last_frame_time_sec);
                    if frame_times.len() > 5 {
                        frame_times.pop_front();
                    }
                    let approx_frame_time =
                        frame_times.iter().sum::<f32>() / frame_times.len() as f32;
                    let remaining_time_sec = approx_frame_time * remaining_frames as f32;

                    *stats.lock().unwrap() = Some(ComputeStats {
                        remaining_time_sec,
                        last_frame_time_sec,
                        last_frame_substeps: 0, // TODO
                    });
                }

                #[cfg(feature = "profile")]
                {
                    let mut buf = std::io::BufWriter::new(Vec::new());
                    coarse_prof::write(&mut buf).unwrap();
                    info!("{}", String::from_utf8(buf.into_inner().unwrap()).unwrap());
                    coarse_prof::reset();
                }

                info!("done computing {}", number_of_frames.get());

                Ok(())
            }))
        };

        Ok(Self {
            stats,
            harness,
            thread,
        })
    }

    pub fn running(&self) -> bool {
        self.thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
    }

    pub fn poll(&mut self) -> Result<Vec<ReportInfo>, Error> {
        let Some(thread) = self.thread.take() else {
            return Ok(Default::default());
        };
        if thread.is_finished() {
            thread
                .join()
                .map_err(|e| Error::ComputePanic(panic_payload_to_string(e)))??;
            return Ok(Default::default());
        }
        self.thread = Some(thread);
        Ok(self.harness.get_infos()?)
    }

    pub fn stats(&self) -> Result<Option<ComputeStats>, Error> {
        Ok(self
            .stats
            .lock()
            .map_err(|_| Error::ComputeStatsMutexPoisoned)?
            .clone())
    }
}

impl Drop for ComputeThread {
    fn drop(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        self.harness.cancel();
        if let Err(e) = thread.join() {
            tracing::error!(payload = panic_payload_to_string(e), "Compute Panic");
        }
    }
}
