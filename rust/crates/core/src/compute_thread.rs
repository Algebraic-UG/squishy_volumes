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
use squishy_volumes_gpu::{PipelinePart as _, wgpu};
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
                        current_state.particles.sort_map.len().max(1000) as u64 * 2048,
                        "main allocator",
                        true,
                    )?;
                    gpu_context.setup_indirect_allocator(2048, "indirect allocator", true)?;

                    info!("setting up GPU state");
                    Some(current_state.to_gpu_state(&mut phase_input, gpu_context))
                } else {
                    None
                };

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
                        let indirect_particles = next_input.indirect_particles.clone();

                        let vertex_positions_start = next_input.vertex_positions_start.clone();
                        let vertex_positions_end = next_input.vertex_positions_end.clone();
                        let vertex_triangle_offsets = next_input.vertex_triangle_offsets.clone();
                        let vertex_triangle_lists = next_input.vertex_triangle_lists.clone();

                        let triangle_indices = next_input.triangle_indices.clone();
                        let triangle_collider = next_input.triangle_collider.clone();
                        let triangle_opposites = next_input.triangle_opposites.clone();
                        let triangle_frictions = next_input.triangle_frictions.clone();

                        let bvh = next_input.bvh.clone();

                        let mut encoder = gpu_context
                            .device()
                            .create_command_encoder(&Default::default());

                        let mut recorded_steps = 0;
                        while current_state.time() < next_stored_frame_time {
                            if !run.load(Ordering::Relaxed) {
                                return Ok(());
                            }

                            let squishy_volumes_gpu::step::Output {
                                indices_out,
                                collider_bits_out,
                                masses_out,
                                initial_volumes_out,
                                parameters_out,
                                positions_out,
                                position_gradients_out,
                                velocities_out,
                                velocity_gradients_out,
                            } = pipeline_part.record(
                                &mut gpu_context,
                                &mut (&mut encoder).into(),
                                next_input,
                                squishy_volumes_gpu::step::Parameters {
                                    factor: ((current_state.time()
                                        * phase_input.consts.frames_per_second as f64)
                                        % 1.) as f32,
                                },
                            )?;
                            next_input = squishy_volumes_gpu::step::Input {
                                indirect_particles: indirect_particles.clone(),
                                indices_in: indices_out.clone(),
                                collider_bits_in: collider_bits_out.clone(),
                                masses_in: masses_out,
                                initial_volumes_in: initial_volumes_out,
                                parameters_in: parameters_out,
                                positions_in: positions_out.clone(),
                                position_gradients_in: position_gradients_out.clone(),
                                velocities_in: velocities_out.clone(),
                                velocity_gradients_in: velocity_gradients_out.clone(),

                                vertex_positions_start: vertex_positions_start.clone(),
                                vertex_positions_end: vertex_positions_end.clone(),
                                vertex_triangle_offsets: vertex_triangle_offsets.clone(),
                                vertex_triangle_lists: vertex_triangle_lists.clone(),

                                triangle_indices: triangle_indices.clone(),
                                triangle_collider: triangle_collider.clone(),
                                triangle_opposites: triangle_opposites.clone(),
                                triangle_frictions: triangle_frictions.clone(),
                                bvh: bvh.clone(),
                            };
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
                                next_input.indices_in.clone(),
                                next_input.positions_in.clone(),
                                next_input.position_gradients_in.clone(),
                                next_input.velocities_in.clone(),
                            ],
                        );
                        downloads.copy(&mut encoder);

                        info!("submit");
                        gpu_context.queue().submit([encoder.finish()]);

                        let downloads = downloads.prep();

                        info!("wait");
                        gpu_context
                            .device()
                            .poll(wgpu::PollType::wait_indefinitely())
                            .unwrap();

                        info!("download");
                        let [
                            indices_out,
                            positions_out,
                            position_gradients_out,
                            velocities_out,
                        ] = downloads.try_into().unwrap();

                        current_state.particles.sort_map = indices_out
                            .to_vec::<u32>()
                            .into_iter()
                            .map(|i| i as usize)
                            .collect();
                        current_state.particles.positions = positions_out
                            .to_vec::<Vector4<f32>>()
                            .iter()
                            .map(Vector4::xyz)
                            .collect();
                        current_state.particles.position_gradients = position_gradients_out
                            .to_vec::<Matrix4x3<f32>>()
                            .into_iter()
                            .map(|m| m.fixed_view::<3, 3>(0, 0).into())
                            .collect();
                        current_state.particles.velocities = velocities_out
                            .to_vec::<Vector4<f32>>()
                            .iter()
                            .map(Vector4::xyz)
                            .collect();

                        info!("reverse");
                        current_state
                            .particles
                            .reverse_sort_map
                            .resize(current_state.particles.sort_map.len(), 0);
                        for (current, original) in
                            current_state.particles.sort_map.iter().enumerate()
                        {
                            current_state.particles.reverse_sort_map[*original] = current;
                        }

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
