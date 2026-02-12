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
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{JoinHandle, spawn},
    time::Instant,
};

use anyhow::{Context, Result};
use squishy_volumes_api::{T, Task};
use strum::IntoEnumIterator;
use tracing::{debug, info};

use crate::{
    input_file::{InputConsts, InputReader},
    input_interpolation::InputInterpolation,
    phase::{Phase, PhaseInput},
    report::{Report, ReportInfo},
    state::State,
    stats::ComputeStats,
};

use super::cache::Cache;

pub struct ComputeThread {
    pub stats: Arc<Mutex<Option<ComputeStats>>>,

    run: Arc<AtomicBool>,
    report: Report,
    thread: Option<JoinHandle<Result<()>>>,
}

pub struct ComputeThreadSettings {
    pub consts: InputConsts,
    pub input_reader: InputReader,
    pub cache: Arc<Cache>,
    pub time_step: T,
    pub max_time_step: T,
    pub number_of_frames: NonZero<usize>,
    pub next_frame: usize,
    pub adaptive_time_steps: bool,
    pub explicit: bool,
    pub debug_mode: bool,
}

impl ComputeThread {
    pub fn new(
        ComputeThreadSettings {
            consts,
            mut input_reader,
            cache,
            time_step,
            max_time_step,
            number_of_frames,
            mut next_frame,
            adaptive_time_steps,
            explicit,
            debug_mode,
        }: ComputeThreadSettings,
    ) -> Result<Self> {
        info!("starting compute thread");

        let run = Arc::new(AtomicBool::new(true));
        let report = Report::new(ReportInfo {
            name: "Simulating Frames".to_string(),
            completed_steps: next_frame,
            steps_to_completion: number_of_frames,
        });
        let seconds_per_frame = 1. / consts.frames_per_second as f64;
        let stats = Arc::new(Mutex::new(None));
        let thread = {
            let run = run.clone();
            let frame_report = report.clone();
            let stats = stats.clone();
            Some(spawn(move || -> Result<()> {
                let mut current_state = if next_frame == 0 {
                    let state = State::new(
                        run.clone(),
                        frame_report.clone(),
                        input_reader.read_header()?,
                        input_reader.read_frame(0)?,
                    )?;
                    cache.store_frame(state.clone())?;
                    frame_report.step();
                    next_frame += 1;
                    state
                } else {
                    cache.fetch_frame(next_frame - 1)?
                };

                let input_interpolation = InputInterpolation::new(input_reader)?;
                let mut phase_input = PhaseInput {
                    consts,
                    input_interpolation,
                    time_step,
                    max_time_step,
                    time_step_by_velocity: None,
                    time_step_by_deformation: None,
                    time_step_by_isolated: None,
                    time_step_by_sound: None,
                    time_step_by_sound_simple: None,
                    time_step_prior: Default::default(),
                    adaptive_time_steps,
                    explicit,
                    debug_mode,
                };

                let mut frame_times = VecDeque::new();
                while next_frame < number_of_frames.get() {
                    let start_compute_frame = Instant::now();

                    let step_report = frame_report.new_sub(ReportInfo {
                        name: "Simulation Milliseconds to Next Frame".to_string(),
                        completed_steps: 0,
                        steps_to_completion: NonZero::new(
                            ((seconds_per_frame * 1000.) as usize).max(1),
                        )
                        .unwrap(),
                    });

                    let next_stored_frame_time = next_frame as f64 * seconds_per_frame;
                    let mut substeps = 0;
                    while current_state.time() < next_stored_frame_time {
                        let phase_report = step_report.new_sub(ReportInfo {
                            name: "Phases".to_string(),
                            completed_steps: 0,
                            steps_to_completion: NonZero::new(Phase::iter().count()).unwrap(),
                        });
                        loop {
                            if !run.load(Ordering::Relaxed) {
                                return Ok(());
                            }

                            current_state = current_state.next(&mut phase_input)?;
                            phase_report.step();
                            if !run.load(Ordering::Relaxed) {
                                return Ok(());
                            }

                            if current_state.phase() == Phase::default() {
                                break;
                            }
                        }

                        step_report.set_completed(
                            ((current_state.time() % seconds_per_frame) * 1000.) as usize,
                        );
                        substeps += 1;
                    }
                    frame_report.step();

                    cache.store_frame(current_state.clone())?;
                    debug!("computed frame {} of {}", next_frame, number_of_frames);
                    next_frame += 1;

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
                        last_frame_substeps: substeps,
                    });
                }

                Ok(())
            }))
        };

        Ok(Self {
            stats,
            run,
            report: report.as_store(),
            thread,
        })
    }

    pub fn running(&self) -> bool {
        self.thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
    }

    pub fn poll(&mut self) -> Result<Option<Task>> {
        let Some(thread) = self.thread.take() else {
            return Ok(None);
        };
        if thread.is_finished() {
            thread.join().unwrap().context("Compute Fail")?;
            return Ok(None);
        }
        self.thread = Some(thread);
        Ok(self.report.clone().into())
    }
}

impl Drop for ComputeThread {
    fn drop(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        self.run.store(false, Ordering::Relaxed);
        let _ = thread.join().unwrap();
    }
}
