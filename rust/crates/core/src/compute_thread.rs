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
    collections::VecDeque,
    num::NonZero,
    sync::Arc,
    thread::{JoinHandle, spawn},
    time::Instant,
};

use squishy_volumes_cache::{Cache, ComputeStats, Stats};
use squishy_volumes_cpu::{CpuRunParameters, CpuState};
use squishy_volumes_file_frame::{Frame, IoState};
use squishy_volumes_file_input::InputReader;
use squishy_volumes_gpu::{GpuRunParameters, GpuState};
use squishy_volumes_util::panic_payload_to_string;
use squishy_volumes_xpu::{Harness, ReportInfo};
use tracing::info;

#[cfg(feature = "profile")]
use squishy_volumes_util::coarse_prof;

use crate::{
    Panic, SimulationError, StoredCause, initialization::initialize_io_state, simulation_input_path,
};

pub struct ComputeThread {
    harness: Harness,
    thread: Option<JoinHandle<Result<(), SimulationError>>>,
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
    ) -> Result<Self, SimulationError> {
        info!("starting compute thread");

        let mut input_reader = InputReader::new(simulation_input_path(cache.directory()))
            .map_err(SimulationError::StartReadingInput)?;

        let harness = Harness::new("Simulating Frames".to_string(), number_of_frames);
        harness.step_to(next_frame)?;

        let frame_store = cache.start_storing();

        let thread = {
            let harness = harness.clone();
            Some(spawn(move || -> Result<(), SimulationError> {
                info!("compute thread started");
                let io_state = if next_frame == 0 {
                    info!("creating initial state");
                    let io_state = initialize_io_state(&harness, &mut input_reader)?;
                    frame_store
                        .store_frame(
                            Frame {
                                io_state: io_state.clone(),
                                error_chain: None,
                            },
                            make_stats(&io_state, None),
                        )
                        .map_err(SimulationError::StoreFrame)?;
                    next_frame += 1;
                    harness.step()?;
                    io_state
                } else {
                    info!("loading checkpoint");
                    let frame = cache
                        .fetch_frame(next_frame - 1)
                        .map_err(|source| SimulationError::LoadCheckpoint(source.into()))?;
                    if let Some(error_chain) = frame.error_chain {
                        return Err(SimulationError::StoredFrame {
                            frame: next_frame - 1,
                            cause: StoredCause::from_chain(error_chain),
                        });
                    }
                    frame.io_state
                };
                harness.check()?;

                #[allow(clippy::large_enum_variant)]
                enum ComputeState {
                    Cpu(CpuState),
                    Gpu(GpuState),
                }

                let mut compute_state = if let Some(gpu) = gpu {
                    ComputeState::Gpu(GpuState::new(
                        next_frame - 1,
                        io_state,
                        input_reader,
                        gpu,
                        &harness,
                        max_time_step,
                        Some(cache.directory().join("gpu_profile.csv")),
                    )?)
                } else {
                    ComputeState::Cpu(CpuState::new(next_frame - 1, io_state, input_reader)?)
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

                    let result: Result<(), SimulationError>;
                    let last_frame_substeps;
                    let io_state = match &mut compute_state {
                        ComputeState::Cpu(cpu_state) => {
                            let (io_state, cpu_result) = cpu_state.produce_next_state(
                                &harness,
                                CpuRunParameters {
                                    max_time_step,
                                    adaptive_time_steps,
                                    store_grid: true,
                                    store_bvh: true,
                                },
                            )?;
                            result = cpu_result.map_err(SimulationError::from);
                            last_frame_substeps = cpu_state.last_frame_substeps();
                            io_state
                        }
                        ComputeState::Gpu(gpu_state) => {
                            let (io_state, gpu_result) = gpu_state.produce_next_state(
                                &harness,
                                GpuRunParameters {
                                    adaptive_time_steps,
                                    store_grid: true,
                                    store_bvh: true,
                                },
                            )?;
                            result = gpu_result.map_err(SimulationError::from);
                            last_frame_substeps = gpu_state.last_frame_substeps();
                            io_state
                        }
                    };

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

                    let stats = make_stats(
                        &io_state,
                        Some(ComputeStats {
                            remaining_time_sec,
                            last_frame_time_sec,
                            last_frame_substeps,
                        }),
                    );

                    let error_chain = result.as_ref().err().map(|error| StoredCause::chain(error));

                    // store state even if error occured
                    frame_store
                        .store_frame(
                            Frame {
                                io_state,
                                error_chain,
                            },
                            stats,
                        )
                        .map_err(SimulationError::StoreFrame)?;

                    // now check for errors
                    result?;

                    info!("computed frame {} of {}", next_frame, number_of_frames);
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

        Ok(Self { harness, thread })
    }

    pub fn running(&self) -> bool {
        self.thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
    }

    pub fn stop(mut self) -> Result<(), SimulationError> {
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };
        if !thread.is_finished() {
            self.harness.cancel();
        }
        match thread.join().map_err(compute_panic)? {
            Err(SimulationError::Canceled(_)) => Ok(()),
            result => result,
        }
    }

    pub fn poll(&mut self) -> Result<Vec<ReportInfo>, SimulationError> {
        let Some(thread) = self.thread.take() else {
            return Ok(Default::default());
        };
        if thread.is_finished() {
            thread.join().map_err(compute_panic)??;
            return Ok(Default::default());
        }
        self.thread = Some(thread);
        Ok(self.harness.get_infos())
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

fn compute_panic(payload: Box<dyn std::any::Any + Send>) -> SimulationError {
    SimulationError::Panic(Panic {
        thread: "compute",
        message: panic_payload_to_string(payload),
    })
}

fn make_stats(io_state: &IoState, compute: Option<ComputeStats>) -> Stats {
    Stats {
        grid_node_count: io_state
            .grid_nodes
            .as_ref()
            .map(|grid_nodes| grid_nodes.collider_bits.len() as u32),
        compute,
    }
}

#[cfg(test)]
impl ComputeThread {
    pub(crate) fn spawn_work(
        work: impl FnOnce(Harness) -> Result<(), SimulationError> + Send + 'static,
    ) -> Self {
        let harness = Harness::new("test".to_string(), NonZero::new(1).unwrap());
        let thread = {
            let harness = harness.clone();
            Some(spawn(move || work(harness)))
        };
        Self { harness, thread }
    }

    pub(crate) fn until_canceled(harness: Harness) -> Result<(), SimulationError> {
        while !harness.is_canceled() {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        harness.check()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use squishy_volumes_directory_lock::DirectoryLock;
    use squishy_volumes_file_input::{InputHeader, write_test_input};
    use squishy_volumes_util::EnergyError;
    use tempfile::TempDir;

    use super::*;

    fn compute_thread(
        work: impl FnOnce(Harness) -> Result<(), SimulationError> + Send + 'static,
    ) -> ComputeThread {
        ComputeThread::spawn_work(work)
    }

    fn until_canceled(harness: Harness) -> Result<(), SimulationError> {
        ComputeThread::until_canceled(harness)
    }

    #[test]
    fn stopping_a_running_thread_is_not_an_error() {
        let thread = compute_thread(until_canceled);

        assert!(thread.stop().is_ok());
    }

    #[test]
    fn stopping_a_failed_thread_returns_its_error() {
        let thread = compute_thread(|_| {
            Err(SimulationError::StoredFrame {
                frame: 0,
                cause: None,
            })
        });
        while thread.running() {
            std::thread::yield_now();
        }

        assert!(matches!(
            thread.stop(),
            Err(SimulationError::StoredFrame { .. })
        ));
    }

    fn displayed_chain(error: SimulationError) -> Vec<String> {
        anyhow::Error::from(error)
            .chain()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn a_stored_frame_error_displays_like_the_live_one_under_the_frame() {
        let directory = TempDir::new().unwrap();
        write_test_input(
            simulation_input_path(directory.path()),
            InputHeader::test_input([]),
            2,
        );
        let directory_lock = DirectoryLock::new(directory.path().to_path_buf()).unwrap();
        let cache = Arc::new(Cache::new(directory_lock, 0, u64::MAX).unwrap());
        let live_error = || {
            SimulationError::from(squishy_volumes_cpu::Error::Energy(
                EnergyError::PositionGradientNotInvertable,
            ))
        };

        let frame = Frame {
            io_state: IoState::default(),
            error_chain: Some(StoredCause::chain(&live_error())),
        };
        let stats = make_stats(&frame.io_state, None);
        cache.start_storing().store_frame(frame, stats).unwrap();
        while cache.storing() {
            std::thread::yield_now();
        }

        let rebake = ComputeThread::new(ComputeThreadSettings {
            cache: cache.clone(),
            max_time_step: 0.01,
            number_of_frames: NonZero::new(2).unwrap(),
            next_frame: 1,
            gpu: None,
            adaptive_time_steps: true,
        })
        .unwrap();
        while rebake.running() {
            std::thread::yield_now();
        }
        let stored = displayed_chain(rebake.stop().unwrap_err());

        assert_eq!(
            stored[0],
            "frame 0 failed, rebake from an earlier frame to retry"
        );
        assert_eq!(stored[1..], displayed_chain(live_error()));
    }

    #[test]
    fn stopping_a_panicked_thread_reports_the_panic() {
        let thread = compute_thread(|_| panic!("compute thread test panic"));
        while thread.running() {
            std::thread::yield_now();
        }

        assert!(matches!(
            thread.stop(),
            Err(SimulationError::Panic(Panic {
                thread: "compute",
                message,
            })) if message == "compute thread test panic"
        ));
    }

    #[test]
    fn polling_a_panicked_thread_reports_the_panic() {
        let mut thread = compute_thread(|_| panic!("compute thread test panic"));
        while thread.running() {
            std::thread::yield_now();
        }

        assert!(matches!(
            thread.poll(),
            Err(SimulationError::Panic(Panic {
                thread: "compute",
                ..
            }))
        ));
    }
}
