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
use nalgebra::{Matrix4x3, Vector4};
use squishy_volumes_api::{T, Task};
use squishy_volumes_gpu::{
    PipelinePart as _, PositionAndColliderBits, ProfileDataCsvWriter, profiler_output, wgpu,
    wgpu_profiler,
};
use strum::IntoEnumIterator;
use tracing::info;

use crate::{
    input_file::{InputConsts, InputReader},
    input_interpolation::InputInterpolation,
    phase::{Phase, PhaseInput},
    profile,
    report::{Report, ReportInfo},
    state::{GpuState, State},
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
    pub gpu_context: Option<squishy_volumes_gpu::GpuContext>,
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
            gpu_context,
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
                info!("compute thread started");
                let mut current_state = if next_frame == 0 {
                    info!("creating initial state");
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
                    info!("loading checkpoint");
                    cache.fetch_frame(next_frame - 1)?
                };

                let input_interpolation =
                    InputInterpolation::new(input_reader, &consts, next_frame - 1)?;
                let mut phase_input = PhaseInput {
                    consts,
                    input_interpolation,
                    time_step,
                    max_time_step,
                    time_step_by_velocity: None,
                    time_step_by_deformation: None,
                    time_step_by_isolated: None,
                    time_step_by_sound: None,
                    time_step_prior: Default::default(),
                    adaptive_time_steps,
                    explicit,
                };

                let frame_time = current_state.time() * phase_input.consts.frames_per_second as f64;
                // this should be a no-op for all in-between-frame-steps
                phase_input
                    .input_interpolation
                    .load(&phase_input.consts, frame_time.floor() as usize)?;

                let mut gpu_state = if let Some(mut gpu_context) = gpu_context {
                    info!("setting up GPU allocators");
                    gpu_context.setup_allocator(
                        current_state.particles.sort_map.len().max(1000) as u64 * 4096,
                        "main allocator",
                        true,
                    )?;
                    gpu_context.setup_indirect_allocator(2048, "indirect allocator", true)?;

                    info!("setting up GPU state");
                    Some(current_state.to_gpu_state(&mut phase_input, gpu_context))
                } else {
                    None
                };

                let mut profile_data_csv_writer = ProfileDataCsvWriter::new("profile.csv")?;

                let mut frame_times = VecDeque::new();
                while next_frame < number_of_frames.get() {
                    profile!("frame");
                    if !run.load(Ordering::Relaxed) {
                        return Ok(());
                    }

                    let start_compute_frame = Instant::now();
                    let next_stored_frame_time = next_frame as f64 * seconds_per_frame;
                    let mut substeps = 0;

                    if let Some(GpuState {
                        mut gpu_context,
                        pipeline_part,
                        mut next_input,
                    }) = gpu_state
                    {
                        let mut encoder = gpu_context
                            .device()
                            .create_command_encoder(&Default::default());
                        let mut profiler = wgpu_profiler::GpuProfiler::new(
                            gpu_context.device(),
                            Default::default(),
                        )
                        .unwrap();

                        let mut recorded_steps = 0;
                        while current_state.time() < next_stored_frame_time {
                            if !run.load(Ordering::Relaxed) {
                                return Ok(());
                            }
                            let scope = profiler.scope("run_step", &mut encoder);
                            let squishy_volumes_gpu::step::Output { .. } = pipeline_part.record(
                                &mut gpu_context,
                                &mut scope.into(),
                                next_input.clone(),
                                squishy_volumes_gpu::step::Parameters {
                                    factor: ((current_state.time()
                                        * phase_input.consts.frames_per_second as f64)
                                        % 1.) as f32,
                                },
                            )?;
                            current_state.time += time_step as f64;

                            recorded_steps += 1;
                            if recorded_steps > 10 {
                                info!("submit");
                                gpu_context.queue().submit([encoder.finish()]);
                                encoder = gpu_context
                                    .device()
                                    .create_command_encoder(&Default::default());
                                recorded_steps = 0;
                            }
                        }

                        let downloads = squishy_volumes_gpu::DownloadsToHost::new(
                            &gpu_context,
                            [
                                next_input.particle_positions_and_collider_bits.clone(),
                                next_input.particle_position_gradients.clone(),
                                next_input.particle_velocities.clone(),
                            ],
                        );
                        downloads.copy(&mut encoder);

                        profiler.resolve_queries(&mut encoder);

                        info!("submit");
                        gpu_context.queue().submit([encoder.finish()]);

                        let downloads = downloads.prep();
                        profiler.end_frame().unwrap();

                        info!("wait");
                        gpu_context
                            .device()
                            .poll(wgpu::PollType::wait_indefinitely())
                            .unwrap();

                        //profiler_output(&gpu_context, &mut profiler)?;
                        profile_data_csv_writer.write_frame(
                            &gpu_context,
                            &mut profiler,
                            next_frame,
                        )?;

                        info!("download");
                        let [
                            particle_positions_and_collider_bits,
                            particle_position_gradients,
                            particle_velocities,
                        ] = downloads.try_into().unwrap();

                        let particle_positions_and_collider_bits: Vec<PositionAndColliderBits> =
                            particle_positions_and_collider_bits.to_vec();

                        current_state.particles.collider_bits =
                            particle_positions_and_collider_bits
                                .iter()
                                .map(|position_and_bits| position_and_bits.collider_bits)
                                .collect();
                        current_state.particles.positions = particle_positions_and_collider_bits
                            .into_iter()
                            .map(|position_and_bits| position_and_bits.position)
                            .collect();
                        current_state.particles.position_gradients = particle_position_gradients
                            .to_vec::<Matrix4x3<f32>>()
                            .into_iter()
                            .map(|m| m.fixed_view::<3, 3>(0, 0).into())
                            .collect();
                        current_state.particles.velocities = particle_velocities
                            .to_vec::<Vector4<f32>>()
                            .iter()
                            .map(Vector4::xyz)
                            .collect();

                        gpu_state = Some(GpuState {
                            gpu_context,
                            pipeline_part,
                            next_input,
                        });
                    } else {
                        let step_report = frame_report.new_sub(ReportInfo {
                            name: "Simulation Milliseconds to Next Frame".to_string(),
                            completed_steps: 0,
                            steps_to_completion: NonZero::new(
                                ((seconds_per_frame * 1000.) as usize).max(1),
                            )
                            .unwrap(),
                        });

                        while current_state.time() < next_stored_frame_time {
                            profile!("substep");
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
                    }

                    frame_report.step();

                    cache.store_frame(current_state.clone())?;
                    info!("computed frame {} of {}", next_frame, number_of_frames);
                    #[cfg(feature = "profile")]
                    if next_frame == 1 {
                        coarse_prof::reset();
                        info!("profile reset");
                    }
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
                #[cfg(feature = "profile")]
                {
                    let mut buf = std::io::BufWriter::new(Vec::new());
                    coarse_prof::write(&mut buf)?;
                    info!("{}", String::from_utf8(buf.into_inner()?)?);
                    coarse_prof::reset();
                }

                info!("done computing {}", number_of_frames.get());

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
