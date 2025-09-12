// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    num::NonZero,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{JoinHandle, spawn},
};

use anyhow::{Context, Result};
use squishy_volumes_api::Task;
use strum::IntoEnumIterator;
use tracing::{debug, info};

use crate::{
    State,
    report::{Report, ReportInfo},
    simulation::state::{Phase, PhaseInput},
};

use super::cache::Cache;

pub struct ComputeThread {
    run: Arc<AtomicBool>,
    report: Report,
    thread: Option<JoinHandle<Result<()>>>,
}

impl ComputeThread {
    pub fn new(
        cache: Arc<Cache>,
        frames_per_second: usize,
        mut phase_input: PhaseInput,
        number_of_frames: NonZero<usize>,
        mut next_frame: usize,
    ) -> Result<Self> {
        info!("starting compute thread");

        let run = Arc::new(AtomicBool::new(true));
        let report = Report::new(ReportInfo {
            name: "Simulating Frames".to_string(),
            completed_steps: next_frame,
            steps_to_completion: number_of_frames,
        });
        let seconds_per_frame = 1. / frames_per_second as f64;
        let thread = {
            let run = run.clone();
            let frame_report = report.clone();
            Some(spawn(move || -> Result<()> {
                let mut current_state = if next_frame == 0 {
                    let state = State::new(run.clone(), frame_report.clone(), &cache.setup)?;
                    cache.store_frame(state.clone())?;
                    frame_report.step();
                    next_frame += 1;
                    state
                } else {
                    cache.fetch_frame(next_frame - 1)?
                };

                while next_frame < number_of_frames.get() {
                    let step_report = frame_report.new_sub(ReportInfo {
                        name: "Simulation Milliseconds to Next Frame".to_string(),
                        completed_steps: 0,
                        steps_to_completion: NonZero::new(
                            ((seconds_per_frame * 1000.) as usize).max(1),
                        )
                        .unwrap(),
                    });

                    let next_stored_frame_time = next_frame as f64 * seconds_per_frame;
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
                    }
                    frame_report.step();

                    cache.store_frame(current_state.clone())?;
                    debug!("computed frame {} of {}", next_frame, number_of_frames);
                    next_frame += 1;
                }

                Ok(())
            }))
        };

        Ok(Self {
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
