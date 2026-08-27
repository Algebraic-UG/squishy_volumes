// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{mem::swap, num::NonZeroU32, path::PathBuf, time::Duration};

use nalgebra::{Matrix1x3, Matrix3, Matrix4x3, Vector3, Vector4, stack};
use squishy_volumes_file_frame::{IoState, ParticleFlags};
use squishy_volumes_xpu::{FrameInput, Harness};

use crate::{
    particle_parameters::ParticleParametersDevice,
    step::{VariableParticleInput, VariableParticleInputData},
    time_step_limits::TimeStepLimits,
};

use super::*;

pub struct GpuState {
    time: f64,
    max_time_step: f32,
    gpu_context: GpuContext,
    update_flags: UpdateFlags,
    pipeline_part: Step,
    new_flags: Allocation,
    next_input: step::Input,
    max_num_grid_nodes: NonZeroU32,

    steps_per_frame: u32,
    recorded_steps: u32,

    io_state: IoState,
    profile_data_csv_writer: Option<ProfileDataCsvWriter>,
}

pub const BYTES_PER_GRID_NODE: u64 = 300;

impl GpuState {
    pub fn from_io_state(
        gpu: String,
        harness: &Harness,
        frame_input: &FrameInput,
        max_time_step: f32,
        io_state: IoState,
        profiling_output_file: Option<PathBuf>,
    ) -> Result<Self, GpuError> {
        tracing::info!("setting up GPU state");
        let harness = harness.scope("Setting up GPU State".to_string(), 5.try_into().unwrap())?;

        let consts = frame_input.consts();

        let max_num_grid_nodes: NonZeroU32 = (io_state.particles.flags.len() as u32)
            .max(1000)
            .try_into()
            .unwrap();

        tracing::info!(max_num_grid_nodes, "this is the limit for now");

        // This will most likely be wrong the first time
        let steps_per_frame =
            (1. / (consts.frames_per_second as f32 * max_time_step)).ceil() as u32;
        tracing::info!(steps_per_frame, "first estimate");

        let mut gpu_context = GpuContext::new(Some(gpu))?;
        harness.check()?;
        harness.step()?;

        tracing::info!("setting up GPU allocators");
        gpu_context.setup_allocator(
            Some(&harness),
            max_num_grid_nodes.get() as u64 * BYTES_PER_GRID_NODE,
            "main allocator",
            false,
        )?;
        gpu_context.setup_indirect_allocator(2048, "indirect allocator", false)?;
        harness.check()?;
        harness.step()?;

        let dispatch_limit = gpu_context
            .device()
            .limits()
            .max_compute_workgroups_per_dimension
            .try_into()
            .unwrap();
        let workgroup_size = 64.try_into().unwrap(); // TODO: make configurable?

        let update_flags = UpdateFlags::new(
            &mut gpu_context,
            update_flags::Settings {
                workgroup_size,
                dispatch_limit,
            },
        )?;
        let pipeline_part = Step::new(
            &mut gpu_context,
            step::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size: consts.scaled_grid_node_size(),
                frames_per_second: consts.frames_per_second,
                forget_distance: consts.forget_distance(),
                accept_distance: consts.accept_distance(),
                max_time_step,
                time_step_history_length: 10, // TODO: make configurable?
                table_tries: 50,              // TODO: make configurable?
                domain_min: consts.scaled_domain_min().into(),
                domain_max: consts.scaled_domain_max().into(),
            },
        )?;
        harness.check()?;
        harness.step()?;

        let device = gpu_context.device();

        let num_particles = io_state.particles.flags.len();
        tracing::info!(num_particles, "preparing particles for transfer");
        let particle_parameters: Vec<ParticleParametersDevice> = io_state
            .particles
            .parameters
            .iter()
            .map(Into::into)
            .collect();
        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: num_particles as u32,
        });

        let a = frame_input.a();
        let b = frame_input.b().unwrap_or(a);

        let particle_goals_start = a
            .particle_goal_positions()
            .iter()
            .map(|p| p.push(0.))
            .collect::<Vec<_>>();
        let particle_goals_end = b
            .particle_goal_positions()
            .iter()
            .map(|p| p.push(0.))
            .collect::<Vec<_>>();
        harness.check()?;
        harness.step()?;

        tracing::info!("creating particle allocations");

        let time_between_frames =
            (io_state.time % (1. / frame_input.consts().frames_per_second as f64)) as f32;
        let time = Allocation::new(device, "time", &[time_between_frames])?;
        // TODO: interpolate that
        let gravity = Allocation::new(device, "gravity", &[a.gravity().push(0.)])?;

        let new_flags = Allocation::new(device, "new_flags", a.particle_flags())?;
        let indirect_particles = Allocation::new(device, "indirect_particles", &[indirect])?;
        let particle_parameters =
            Allocation::new(device, "particle_parameters", &particle_parameters)?;

        let particle_goals_start =
            Allocation::new(device, "particle_goals_start", &particle_goals_start)?;
        let particle_goals_end =
            Allocation::new(device, "particle_goals_end", &particle_goals_end)?;

        let variable_particle_input = get_variable_particle_input(device, &io_state)?;

        let collider_input = get_collider_input(device, frame_input)?;

        let limits_over_time = Allocation::new(
            device,
            "limits_over_time",
            &vec![TimeStepLimits::default(); steps_per_frame as usize],
        )?;

        let next_input = step::Input {
            time,

            gravity,
            indirect_particles,

            particle_parameters,

            particle_goals_start,
            particle_goals_end,

            variable_particle_input,

            collider_input,

            limits_over_time,
        };

        let profile_data_csv_writer = profiling_output_file
            .map(ProfileDataCsvWriter::new)
            .transpose()?;

        harness.check()?;
        harness.step()?;

        Ok(Self {
            time: io_state.time,
            max_time_step,
            gpu_context,
            update_flags,
            pipeline_part,
            new_flags,
            next_input,
            max_num_grid_nodes,
            recorded_steps: 0,
            steps_per_frame,
            io_state,
            profile_data_csv_writer,
        })
    }
}

fn get_variable_particle_input(
    device: &wgpu::Device,
    io_state: &IoState,
) -> Result<step::VariableParticleInput, GpuError> {
    tracing::info!("preparing variable particle data for transfer");

    let particle_positions_and_collider_bits: Vec<PositionAndColliderBits> = io_state
        .particles
        .positions
        .iter()
        .zip(&io_state.particles.collider_bits)
        .map(|(&position, &collider_bits)| PositionAndColliderBits {
            position: position.into(),
            collider_bits,
        })
        .collect();
    #[allow(clippy::toplevel_ref_arg)]
    let particle_position_gradients: Vec<Matrix4x3<f32>> =
        bytemuck::cast_slice::<_, Matrix3<f32>>(&io_state.particles.position_gradients)
            .iter()
            .map(|m| stack![m; Matrix1x3::zeros()])
            .collect();
    let particle_velocities: Vec<Vector4<f32>> =
        bytemuck::cast_slice::<_, Vector3<f32>>(&io_state.particles.velocities)
            .iter()
            .map(|v| v.push(0.))
            .collect();
    #[allow(clippy::toplevel_ref_arg)]
    let particle_velocity_gradients: Vec<Matrix4x3<f32>> =
        bytemuck::cast_slice::<_, Matrix3<f32>>(&io_state.particles.velocity_gradients)
            .iter()
            .map(|m| stack![m; Matrix1x3::zeros()])
            .collect();

    VariableParticleInput::new(
        device,
        VariableParticleInputData {
            particle_flags: &io_state.particles.flags,
            particle_positions_and_collider_bits: &particle_positions_and_collider_bits,
            particle_position_gradients: &particle_position_gradients,
            particle_velocities: &particle_velocities,
            particle_velocity_gradients: &particle_velocity_gradients,
        },
    )
}

fn get_collider_input(
    device: &wgpu::Device,
    frame_input: &FrameInput,
) -> Result<Option<step::ColliderInput>, GpuError> {
    if frame_input.topology().is_empty() {
        return Ok(None);
    }

    let a = frame_input.a();
    let b = frame_input.b().unwrap_or(a);

    let topology = frame_input.topology();
    let num_triangles = topology.triangle_indices().len();
    let num_vertices = topology.vertex_triangle_lists().len();
    tracing::info!(num_vertices, num_triangles, "preparing mesh for transfer");

    let vertex_positions_start: Vec<Vector4<f32>> =
        a.vertex_positions().iter().map(|p| p.push(0.)).collect();
    let vertex_positions_end: Vec<Vector4<f32>> =
        b.vertex_positions().iter().map(|p| p.push(0.)).collect();
    let vertex_velocities: Vec<Vector4<f32>> = frame_input
        .vertex_velocities()
        .iter()
        .map(|v| v.push(0.))
        .collect();

    let vertex_triangle_lists = topology.vertex_triangle_lists();
    let vertex_triangle_offsets = prefix_sum_on_cpu(
        &vertex_triangle_lists
            .iter()
            .map(|v| v.len() as u32)
            .collect::<Vec<_>>(),
    );
    let vertex_triangle_lists: Vec<u32> = vertex_triangle_lists
        .iter()
        .flat_map(|list| list.iter().cloned())
        .collect();

    tracing::info!("creating mesh allocations");
    let vertex_positions_start =
        Allocation::new(device, "vertex_positions_start", &vertex_positions_start)?;
    let vertex_positions_end =
        Allocation::new(device, "vertex_positions_end", &vertex_positions_end)?;
    let vertex_velocities = Allocation::new(device, "vertex_velocities", &vertex_velocities)?;
    let vertex_triangle_offsets =
        Allocation::new(device, "vertex_triangle_offsets", &vertex_triangle_offsets)?;
    let vertex_triangle_lists = if vertex_triangle_lists.is_empty() {
        tracing::warn!("all vertices are on open edges");
        None
    } else {
        Some(Allocation::new(
            device,
            "vertex_triangle_lists",
            &vertex_triangle_lists,
        )?)
    };

    let triangle_indices =
        Allocation::new(device, "triangle_indices", topology.triangle_indices())?;
    let triangle_collider =
        Allocation::new(device, "triangle_collider", topology.triangle_collider())?;
    let triangle_opposites =
        Allocation::new(device, "triangle_opposites", topology.triangle_opposites())?;

    // TODO: interpolate that
    let triangle_frictions = Allocation::new(device, "triangle_frictions", a.triangle_frictions())?;
    let triangle_dampings = Allocation::new(device, "triangle_dampings", a.triangle_dampings())?;

    let num_bvh_levels = frame_input.bvh().level();
    let num_bvh_nodes = frame_input.bvh().nodes().len();
    tracing::info!(num_bvh_levels, num_bvh_nodes, "creating bvh allocations");
    let bvh = BoundingVolumeHierarchyAllocations::new(
        device,
        frame_input.consts().leaf_size,
        frame_input.bvh(),
    )?;

    Ok(Some(step::ColliderInput {
        vertex_positions_start,
        vertex_positions_end,
        vertex_velocities,
        vertex_triangle_offsets,
        vertex_triangle_lists,
        triangle_indices,
        triangle_collider,
        triangle_opposites,
        triangle_frictions,
        triangle_dampings,
        bvh,
    }))
}

pub struct GpuRunParameters {
    pub target_time: f64,
    pub store_grid: bool,
}

impl GpuState {
    pub fn produce_next_state(
        &mut self,
        harness: &squishy_volumes_xpu::Harness,
        frame_input: &mut squishy_volumes_xpu::FrameInput,
        GpuRunParameters {
            target_time,
            store_grid,
        }: GpuRunParameters,
    ) -> Result<(squishy_volumes_file_frame::IoState, Result<(), GpuError>), GpuError> {
        squishy_volumes_util::profile!("produce_next_state");

        if self.time >= target_time {
            return Ok((self.io_state.clone(), Ok(())));
        }

        let mut encoder = self
            .gpu_context
            .device()
            .create_command_encoder(&Default::default());

        self.record_update_flags(&mut encoder)?;

        let mut profiler =
            wgpu_profiler::GpuProfiler::new(self.gpu_context.device(), Default::default()).unwrap();

        let mut redo_frame = false;
        let mut buffered_error = Ok(());
        let mut mapped_downloads;

        self.recorded_steps = 0;
        loop {
            let output = self.record_steps(harness, &mut encoder, &profiler)?;

            let downloads = Downloads::new(self, store_grid, output);
            downloads.copy(&mut encoder);

            profiler.resolve_queries(&mut encoder);

            tracing::info!("submit final");
            let mut tmp = self
                .gpu_context
                .device()
                .create_command_encoder(&Default::default());
            swap(&mut encoder, &mut tmp);
            self.gpu_context.queue().submit([tmp.finish()]);

            let downloads_ready = downloads.prep();

            profiler.end_frame().unwrap();

            self.prepare_for_next_frame(frame_input)?;

            self.wait_for_gpu(harness)?;

            tracing::info!("download");

            mapped_downloads = downloads_ready.into_mapped()?;

            let num_grid_nodes = mapped_downloads.indirect_nodes.len;
            tracing::info!(self.max_num_grid_nodes, num_grid_nodes);

            let result = mapped_downloads.status.to_result(&self.gpu_context);
            self.gpu_context.reset_status()?;
            match result {
                Ok(_) => {
                    tracing::info!("Frame wasn't finished");
                    self.steps_per_frame *= 2;
                    let limits_over_time = Allocation::new(
                        self.gpu_context.device(),
                        "limits_over_time",
                        &vec![TimeStepLimits::default(); self.steps_per_frame as usize],
                    )?;
                    encoder.copy_buffer_to_buffer(
                        self.next_input.limits_over_time.buffer(),
                        self.next_input.limits_over_time.offset(),
                        limits_over_time.buffer(),
                        limits_over_time.offset(),
                        Some(self.next_input.limits_over_time.size().get()),
                    );
                    self.next_input.limits_over_time = limits_over_time;

                    continue;
                }
                Err(GpuError::Shader(GpuShaderError::IndirectLimitExceeded {
                    reporting_shader,
                })) => {
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
                error @ Err(GpuError::Shader(GpuShaderError::ParticleCloseToInverted {
                    reporting_shader,
                })) => {
                    tracing::warn!(reporting_shader, "A particle is too close to inversion.");
                    buffered_error = error;
                    break;
                }
                Err(GpuError::Shader(GpuShaderError::FrameTimeReached)) => {
                    break;
                }
                x => x?,
            };
        }

        if redo_frame {
            if self.max_num_grid_nodes.get() as usize >= self.io_state.particles.flags.len() * 27 {
                return Err(GpuError::MaxGridNodesExceeded);
            }

            self.time = self.io_state.time;
            self.max_num_grid_nodes = (self.max_num_grid_nodes.get() * 2).try_into().unwrap();
            tracing::warn!(self.max_num_grid_nodes, "The frame needs to be redone");
            frame_input.load(frame_input.frame() - 1)?;
            self.new_flags = Allocation::new(
                self.gpu_context.device(),
                "new_flags",
                frame_input.a().particle_flags(),
            )?;
            self.next_input.gravity = Allocation::new(
                self.gpu_context.device(),
                "gravity",
                &[frame_input.a().gravity().push(0.)],
            )?;
            self.next_input.collider_input =
                get_collider_input(self.gpu_context.device(), frame_input)?;
            self.next_input.variable_particle_input =
                get_variable_particle_input(self.gpu_context.device(), &self.io_state)?;
            self.gpu_context.resize_allocator(
                self.max_num_grid_nodes.get() as u64 * BYTES_PER_GRID_NODE,
                false,
            )?;
            return self.produce_next_state(
                harness,
                frame_input,
                GpuRunParameters {
                    target_time,
                    store_grid,
                },
            );
        }

        /* TODO: need to get the times somehow
        if let Some(profile_data_csv_writer) = self.profile_data_csv_writer.as_mut() {
            profile_data_csv_writer.write_frame(&self.gpu_context, &mut profiler, &times)?;
        }
        */

        // TODO: This must be from downloads
        self.io_state.time = self.time;

        update_io_state(&mut self.io_state, mapped_downloads);

        Ok((self.io_state.clone(), buffered_error))
    }

    fn record_update_flags(&mut self, encoder: &mut wgpu::CommandEncoder) -> Result<(), GpuError> {
        // This has to happen just once per frame
        // It doesn't fit with our other profiling stuff
        self.update_flags.record(
            &mut self.gpu_context,
            &mut encoder.into(),
            update_flags::Input {
                new_flags: self.new_flags.clone(),
                flags: self
                    .next_input
                    .variable_particle_input
                    .particle_flags
                    .clone(),
            },
            update_flags::Parameters,
        )?;
        Ok(())
    }

    fn record_steps(
        &mut self,
        harness: &Harness,
        encoder: &mut wgpu::CommandEncoder,
        profiler: &wgpu_profiler::GpuProfiler,
    ) -> Result<step::Output, GpuError> {
        loop {
            harness.check()?;
            let scope = profiler.scope("run_step", encoder);
            let output = self.pipeline_part.record(
                &mut self.gpu_context,
                &mut scope.into(),
                self.next_input.clone(),
                step::Parameters {
                    max_num_grid_nodes: self.max_num_grid_nodes,
                    factor: 0., // TODO: this won't be needed
                    current_step: self.recorded_steps,
                },
            )?;
            self.recorded_steps += 1;

            if self.recorded_steps == self.steps_per_frame || self.recorded_steps.is_multiple_of(10)
            {
                tracing::info!("submit");
                let mut tmp = self
                    .gpu_context
                    .device()
                    .create_command_encoder(&Default::default());
                swap(encoder, &mut tmp);
                self.gpu_context.queue().submit([tmp.finish()]);
            }

            if self.recorded_steps == self.steps_per_frame {
                break Ok(output);
            }
        }
    }

    fn prepare_for_next_frame(&mut self, frame_input: &mut FrameInput) -> Result<(), GpuError> {
        tracing::info!("prepare next frame input");
        frame_input.load(frame_input.frame() + 1)?;

        let b = frame_input.b().unwrap_or(frame_input.a());

        let particle_goals_end = b
            .particle_goal_positions()
            .iter()
            .map(|p| p.push(0.))
            .collect::<Vec<_>>();

        self.new_flags =
            Allocation::new(self.gpu_context.device(), "new_flags", b.particle_flags())?;
        self.next_input.particle_goals_start = self.next_input.particle_goals_end.clone();
        self.next_input.particle_goals_end = Allocation::new(
            self.gpu_context.device(),
            "particle_goals_end",
            &particle_goals_end,
        )?;

        // TODO: interpolate
        self.next_input.gravity = Allocation::new(
            self.gpu_context.device(),
            "gravity",
            &[frame_input.a().gravity().push(0.)],
        )?;

        if let Some(collider_input) = self.next_input.collider_input.as_mut() {
            let vertex_positions_end: Vec<Vector4<f32>> =
                b.vertex_positions().iter().map(|p| p.push(0.)).collect();
            let vertex_velocities: Vec<Vector4<f32>> = frame_input
                .vertex_velocities()
                .iter()
                .map(|v| v.push(0.))
                .collect();
            collider_input.vertex_positions_start = collider_input.vertex_positions_end.clone();
            collider_input.vertex_positions_end = Allocation::new(
                self.gpu_context.device(),
                "vertex_positions_end",
                &vertex_positions_end,
            )?;
            collider_input.vertex_velocities = Allocation::new(
                self.gpu_context.device(),
                "vertex_velocities",
                &vertex_velocities,
            )?;

            collider_input.bvh = BoundingVolumeHierarchyAllocations::new(
                self.gpu_context.device(),
                frame_input.consts().leaf_size,
                frame_input.bvh(),
            )?;
        }

        Ok(())
    }

    fn wait_for_gpu(&self, harness: &Harness) -> Result<(), GpuError> {
        tracing::info!("waiting on GPU");
        loop {
            match self.gpu_context.device().poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_millis(100)),
            }) {
                Ok(_) => break Ok(()),
                Err(wgpu::PollError::Timeout) => {
                    harness.check()?;
                }
                error => {
                    error?;
                }
            }
        }
    }
}

struct Downloads {
    always: DownloadsToHost,
    grid: Option<DownloadsToHost>,
}

impl Downloads {
    fn new(gpu_state: &GpuState, store_grid: bool, output: step::Output) -> Self {
        let always = DownloadsToHost::new(
            &gpu_state.gpu_context,
            [
                gpu_state.gpu_context.status(),
                output.indirect_nodes,
                gpu_state
                    .next_input
                    .variable_particle_input
                    .particle_flags
                    .clone(),
                gpu_state
                    .next_input
                    .variable_particle_input
                    .particle_positions_and_collider_bits
                    .clone(),
                gpu_state
                    .next_input
                    .variable_particle_input
                    .particle_position_gradients
                    .clone(),
                gpu_state
                    .next_input
                    .variable_particle_input
                    .particle_velocities
                    .clone(),
            ],
        );
        let grid = store_grid.then(|| {
            DownloadsToHost::new(
                &gpu_state.gpu_context,
                [output.node_ids_and_collider_bits, output.node_momentums],
            )
        });
        Self { always, grid }
    }

    fn copy(&self, encoder: &mut wgpu::CommandEncoder) {
        self.always.copy(encoder);
        if let Some(downloads_grid) = self.grid.as_ref() {
            downloads_grid.copy(encoder);
        }
    }

    fn prep(&self) -> DownloadsReady<'_> {
        let always = self.always.prep();
        let grid = self.grid.as_ref().map(DownloadsToHost::prep);
        DownloadsReady { always, grid }
    }
}

struct DownloadsReady<'a> {
    always: DownloadsToHostReady<'a>,
    grid: Option<DownloadsToHostReady<'a>>,
}

impl DownloadsReady<'_> {
    fn into_mapped(self) -> Result<MappedDownloads, GpuError> {
        let [
            status,
            indirect_nodes,
            particle_flags,
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
        ] = self.always.try_into().unwrap();

        let grid = self
            .grid
            .map(|grid| {
                let [node_ids_and_collider_bits, node_momentums] = grid.try_into().unwrap();
                Ok::<_, GpuError>(MappedDownloadsGrid {
                    node_ids_and_collider_bits: node_ids_and_collider_bits.to_vec()?,
                    node_momentums: node_momentums.to_vec()?,
                })
            })
            .transpose()?;

        Ok(MappedDownloads {
            status: status.to_vec()?[0],
            indirect_nodes: indirect_nodes.to_vec()?[0],
            particle_flags: particle_flags.to_vec()?,
            particle_positions_and_collider_bits: particle_positions_and_collider_bits.to_vec()?,
            particle_position_gradients: particle_position_gradients.to_vec()?,
            particle_velocities: particle_velocities.to_vec()?,
            grid,
        })
    }
}

struct MappedDownloads {
    status: GpuStatus,
    indirect_nodes: Indirect,
    particle_flags: Vec<ParticleFlags>,
    particle_positions_and_collider_bits: Vec<PositionAndColliderBits>,
    particle_position_gradients: Vec<Matrix4x3<f32>>,
    particle_velocities: Vec<Vector4<f32>>,

    grid: Option<MappedDownloadsGrid>,
}

struct MappedDownloadsGrid {
    node_ids_and_collider_bits: Vec<NodeIdAndColliderBits>,
    node_momentums: Vec<Vector4<f32>>,
}

fn update_io_state(
    io_state: &mut IoState,
    MappedDownloads {
        status: _,
        indirect_nodes,
        particle_flags,
        particle_positions_and_collider_bits,
        particle_position_gradients,
        particle_velocities,
        grid,
    }: MappedDownloads,
) {
    io_state.particles.flags = particle_flags;
    io_state.particles.collider_bits = particle_positions_and_collider_bits
        .iter()
        .map(|position_and_bits| position_and_bits.collider_bits)
        .collect();
    io_state.particles.positions = particle_positions_and_collider_bits
        .into_iter()
        .map(|position_and_bits| position_and_bits.position.into())
        .collect();
    io_state.particles.position_gradients = particle_position_gradients
        .into_iter()
        .map(|m| m.fixed_view::<3, 3>(0, 0).into())
        .collect();
    io_state.particles.velocities = particle_velocities
        .into_iter()
        .map(|v| v.xyz().into())
        .collect();

    io_state.grid_nodes = grid.map(
        |MappedDownloadsGrid {
             node_ids_and_collider_bits,
             node_momentums,
         }| {
            let num_grid_nodes = indirect_nodes.len as usize;
            let node_ids = node_ids_and_collider_bits
                .iter()
                .take(num_grid_nodes)
                .map(|node_id_and_collider_bits| node_id_and_collider_bits.node_id.into())
                .collect();
            let collider_bits = node_ids_and_collider_bits
                .iter()
                .take(num_grid_nodes)
                .map(|node_id_and_collider_bits| node_id_and_collider_bits.collider_bits)
                .collect();
            let masses = node_momentums
                .iter()
                .take(num_grid_nodes)
                .map(|momentum| momentum.w)
                .collect();
            let velocites = node_momentums
                .iter()
                .take(num_grid_nodes)
                .map(|momentum| {
                    if momentum.w != 0. {
                        momentum.xyz() / momentum.w
                    } else {
                        Vector3::zeros()
                    }
                    .into()
                })
                .collect();
            squishy_volumes_file_frame::GridNodes {
                node_ids,
                collider_bits,
                masses,
                velocites,
            }
        },
    );
}
