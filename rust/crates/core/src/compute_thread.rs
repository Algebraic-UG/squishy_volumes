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
use squishy_volumes_xpu::{FrameInput, Harness, ReportInfo};
use tracing::info;

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

    pub gpu: bool,
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

                enum ComputeState {
                    Cpu(CpuState),
                    Gpu,
                }

                let mut compute_state = if gpu {
                    todo!()
                } else {
                    ComputeState::Cpu(CpuState::from_io_state(io_state)?)
                };

                let mut frame_times = VecDeque::new();
                while next_frame < number_of_frames.get() {
                    harness.check()?;

                    let start_compute_frame = Instant::now();

                    frame_input.load(next_frame - 1)?;

                    let target_time = next_frame as f64 / consts.frames_per_second as f64;

                    let io_state = match &mut compute_state {
                        ComputeState::Cpu(cpu_state) => cpu_state.produce_next_state(
                            &harness,
                            &frame_input,
                            CpuRunParameters {
                                target_time,
                                max_time_step,
                                adaptive_time_steps,
                                store_grid: true,
                            },
                        )?,
                        ComputeState::Gpu => todo!(),
                    };

                    cache.store_frame(io_state).map_err(Error::StoreError)?;
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

                info!("done computing {}", number_of_frames.get());

                Ok(())
            }))
        };

        /*
                        phase_input
                            .input_interpolation
                            .load(&phase_input.consts, phase_input.next_frame)?;

                        let mut gpu_state = gpu_context
                            .map(|gpu_context| current_state.to_gpu_state(&mut phase_input, gpu_context))
                            .transpose()?;

                        let mut profile_data_csv_writer = ProfileDataCsvWriter::new("profile.csv")?;

                        let mut frame_times = VecDeque::new();
                        while phase_input.next_frame < number_of_frames.get() {
                            profile!("frame");
                            if !run.load(Ordering::Relaxed) {
                                return Ok(());
                            }

                            let start_compute_frame = Instant::now();
                            let next_stored_frame_time = phase_input.next_frame as f64 * seconds_per_frame;
                            let mut substeps = 0;

                            if let Some(gpu_state) = gpu_state.as_mut() {
                                ComputeFrameGPU {
                                    run: run.clone(),
                                    gpu_state,
                                    current_state: &mut current_state,
                                    next_stored_frame_time,
                                    phase_input: &mut phase_input,
                                    profile_data_csv_writer: &mut profile_data_csv_writer,
                                }
                                .run()?;
                            } else {
                                ComputeFrameCPU {
                                    run: run.clone(),
                                    frame_report: &frame_report,
                                    seconds_per_frame,
                                    current_state: &mut current_state,
                                    next_stored_frame_time,
                                    phase_input: &mut phase_input,
                                    substeps: &mut substeps,
                                }
                                .run()?;
                            }

                            if !run.load(Ordering::Relaxed) {
                                return Ok(());
                            }

                            frame_report.step();

                            cache.store_frame(current_state.clone())?;
                            info!(
                                "computed frame {} of {}",
                                phase_input.next_frame, number_of_frames
                            );
                            #[cfg(feature = "profile")]
                            if next_frame == 1 {
                                coarse_prof::reset();
                                info!("profile reset");
                            }
                            phase_input.next_frame += 1;

                            let last_frame_time_sec = start_compute_frame.elapsed().as_secs_f32();
                            let remaining_frames = number_of_frames.get() - phase_input.next_frame;

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
        */

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
            thread.join().map_err(|_| Error::ComputePanic)??;
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

/*

struct ComputeFrameGPU<'a> {
    run: Arc<AtomicBool>,
    gpu_state: &'a mut GpuState,
    current_state: &'a mut State,
    next_stored_frame_time: f64,
    phase_input: &'a mut PhaseInput,
    profile_data_csv_writer: &'a mut ProfileDataCsvWriter,
}

impl ComputeFrameGPU<'_> {
    pub fn run(mut self) -> Result<()> {
        let Self {
            ref run,
            gpu_state:
                GpuState {
                    gpu_context,
                    pipeline_part,
                    next_input,
                    max_num_grid_nodes,
                },
            ref mut current_state,
            next_stored_frame_time,
            ref mut phase_input,
            ref mut profile_data_csv_writer,
        } = self;
        if current_state.time() >= next_stored_frame_time {
            tracing::warn!("nothing do to for this frame");
            return Ok(());
        }
        let state_start_time = current_state.time();

        let mut encoder = gpu_context
            .device()
            .create_command_encoder(&Default::default());
        let mut profiler =
            wgpu_profiler::GpuProfiler::new(gpu_context.device(), Default::default()).unwrap();

        let mut recorded_steps = 0;
        let indirect_nodes = loop {
            if !run.load(Ordering::Relaxed) {
                return Ok(());
            }
            let scope = profiler.scope("run_step", &mut encoder);
            let squishy_volumes_gpu::step::Output { indirect_nodes, .. } = pipeline_part.record(
                gpu_context,
                &mut scope.into(),
                next_input.clone(),
                squishy_volumes_gpu::step::Parameters {
                    max_num_grid_nodes: *max_num_grid_nodes,
                    factor: current_state.frame_factor(phase_input)?,
                },
            )?;
            current_state.time += phase_input.time_step as f64;

            recorded_steps += 1;
            if recorded_steps > 10 {
                info!("submit");
                gpu_context.queue().submit([encoder.finish()]);
                encoder = gpu_context
                    .device()
                    .create_command_encoder(&Default::default());
                recorded_steps = 0;
            }

            if current_state.time() >= next_stored_frame_time {
                break indirect_nodes;
            }
        };

        let downloads = squishy_volumes_gpu::DownloadsToHost::new(
            gpu_context,
            [
                next_input
                    .variable_particle_input
                    .particle_positions_and_collider_bits
                    .clone(),
                next_input
                    .variable_particle_input
                    .particle_position_gradients
                    .clone(),
                next_input
                    .variable_particle_input
                    .particle_velocities
                    .clone(),
                indirect_nodes,
                gpu_context.status(),
            ],
        );
        downloads.copy(&mut encoder);

        profiler.resolve_queries(&mut encoder);

        info!("submit");
        gpu_context.queue().submit([encoder.finish()]);

        let downloads_ready = downloads.prep();
        profiler.end_frame().unwrap();

        info!("prepare next frame input");
        phase_input
            .input_interpolation
            .load(&phase_input.consts, phase_input.next_frame)?;

        let b = phase_input
            .input_interpolation
            .b()
            .unwrap_or(phase_input.input_interpolation.a());

        let particle_flags: Vec<squishy_volumes_gpu::particle_parameters::Flags> = current_state
            .particles
            .parameters
            .iter()
            .map(crate::state::translate_particle_parameters)
            .zip(b.particle_flags())
            .map(|(p, input_flags)| {
                let mut flags = (&p).into();
                if input_flags.contains(crate::ParticleFlags::HasGoal) {
                    flags |= squishy_volumes_gpu::particle_parameters::Flags::HAS_GOAL;
                }
                flags
            })
            .collect();
        next_input.variable_particle_input.particle_flags =
            Allocation::new(gpu_context.device(), "particle_flags", &particle_flags)?;
        next_input.particle_goals_start = next_input.particle_goals_end.clone();
        let particle_goals_end = b
            .particle_goal_positions()
            .iter()
            .map(|p| p.push(0.))
            .chain(repeat(Vector4::zeros()))
            .take(current_state.particles.sort_map.len())
            .collect::<Vec<_>>();
        next_input.particle_goals_end = Allocation::new(
            gpu_context.device(),
            "particle_goals_end",
            &particle_goals_end,
        )?;

        if let Some(collider_input) = next_input.collider_input.as_mut() {
            collider_input.vertex_positions_start = collider_input.vertex_positions_end.clone();
            let vertex_positions_end: Vec<Vector4<f32>> =
                b.vertex_positions().iter().map(|p| p.push(0.)).collect();
            collider_input.vertex_positions_end = Allocation::new(
                gpu_context.device(),
                "vertex_positions_end",
                &vertex_positions_end,
            )?;

            collider_input.bvh = squishy_volumes_gpu::BoundingVolumeHierarchyAllocations::new(
                gpu_context.device(),
                phase_input.consts.leaf_size,
                phase_input.input_interpolation.bvh(),
            )?;
        }

        info!("wait");

        loop {
            match gpu_context.device().poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_millis(100)),
            }) {
                Ok(_) => break,
                Err(wgpu::PollError::Timeout) => {
                    if !run.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                }
                Err(e) => {
                    bail!("while waiting on GPU: {e}");
                }
            }
        }

        //profiler_output(&gpu_context, &mut profiler)?;
        profile_data_csv_writer.write_frame(gpu_context, &mut profiler, phase_input.next_frame)?;

        info!("download");
        let [
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            indirect_nodes_download,
            status,
        ] = downloads_ready.try_into().unwrap();

        let num_grid_nodes =
            indirect_nodes_download.to_vec::<squishy_volumes_gpu::Indirect>()[0].len;
        tracing::info!(max_num_grid_nodes, num_grid_nodes);

        let mut redo_frame = false;
        match status.to_vec::<GpuStatus>()[0].to_result(gpu_context) {
            Err(GpuError::Shader(GpuShaderError::IndirectLimitExceeded { reporting_shader })) => {
                tracing::warn!(
                    reporting_shader,
                    "The number of grid nodes is larger than expected."
                );
                redo_frame = true;
            }
            Err(GpuError::Shader(GpuShaderError::TableTriesExceeded { reporting_shader })) => {
                tracing::warn!(reporting_shader, "The hash table appears to be too small.");
                redo_frame = true;
            }
            x => x?,
        };
        gpu_context.reset_status()?;

        if redo_frame {
            drop(downloads);
            current_state.time = state_start_time;
            let grid_node_cap = current_state.particles.sort_map.len() * 27;
            anyhow::ensure!(
                (max_num_grid_nodes.get() as usize) < grid_node_cap,
                "theoretical max grid nodes exceeded"
            );
            *max_num_grid_nodes = (max_num_grid_nodes.get() * 2)
                .max(grid_node_cap as u32)
                .try_into()
                .unwrap();
            tracing::warn!(max_num_grid_nodes, "The frame needs to be redone");
            next_input.collider_input =
                State::get_collider_input(phase_input, gpu_context.device())?;
            next_input.variable_particle_input =
                current_state.get_variable_particle_input(phase_input, gpu_context.device())?;
            gpu_context
                .resize_allocator(max_num_grid_nodes.get() as u64 * BYTES_PER_GRID_NODE, false)?;
            return self.run();
        }

        let particle_positions_and_collider_bits: Vec<PositionAndColliderBits> =
            particle_positions_and_collider_bits.to_vec();

        current_state.particles.collider_bits = particle_positions_and_collider_bits
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

        Ok(())
    }
}

struct ComputeFrameCPU<'a> {
    run: Arc<AtomicBool>,
    frame_report: &'a Report,
    seconds_per_frame: f64,
    current_state: &'a mut State,
    next_stored_frame_time: f64,
    phase_input: &'a mut PhaseInput,
    substeps: &'a mut usize,
}

impl ComputeFrameCPU<'_> {
    pub fn run(self) -> Result<()> {
        let Self {
            run,
            frame_report,
            seconds_per_frame,
            current_state,
            next_stored_frame_time,
            phase_input,
            substeps,
        } = self;
        let step_report = frame_report.new_sub(ReportInfo {
            name: "Simulation Milliseconds to Next Frame".to_string(),
            completed_steps: 0,
            steps_to_completion: NonZero::new(((seconds_per_frame * 1000.) as usize).max(1))
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

                *current_state = take(current_state).next(phase_input)?;
                phase_report.step();
                if !run.load(Ordering::Relaxed) {
                    return Ok(());
                }

                if current_state.phase() == Phase::default() {
                    break;
                }
            }

            step_report
                .set_completed(((current_state.time() % seconds_per_frame) * 1000.) as usize);
            *substeps += 1;
        }
        Ok(())
    }
}

*/

impl Drop for ComputeThread {
    fn drop(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        self.harness.cancel();
        // TODO try to get string from error
        if let Err(_) = thread.join() {
            tracing::error!("Compute Panic, could be out of memory, please consult logs.");
        }
    }
}
