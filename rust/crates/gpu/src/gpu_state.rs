// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{mem::swap, num::NonZeroU32, path::PathBuf, time::Duration};

use nalgebra::{Matrix1x3, Matrix3, Matrix4x3, Vector3, Vector4, stack};
use squishy_volumes_file_frame::IoState;
use squishy_volumes_file_input::{
    BulkAttribute, FrameBulkParticles, FrameVerifcationError, InputRangeParticles,
};
use squishy_volumes_util::{AnimatedGlobals, ParticleFlags};
use squishy_volumes_xpu::{FrameInput, FrameInputError, Harness};

use crate::{
    step::{VariableParticleInput, VariableParticleInputData},
    time_step_limits::TimeStepLimits,
};

use super::*;

struct PendingInput {
    animated_globals: AnimatedGlobals,
    globals: Allocation,
    next_goal_positions: Allocation,
    next_collider_input: Option<step::ColliderInput>,
    particle_updates: Vec<PendingParticleUpdate>,
}

struct PendingParticleUpdate {
    attribute: FrameBulkParticles,
    allocation: Allocation,
    offset: u32,
}

pub struct GpuState {
    time: f64,
    animated_globals: AnimatedGlobals,

    gpu_context: GpuContext,

    update_flags: UpdateFlags,
    pipeline_part: Step,

    step_input: step::Input,

    pending_input: Option<PendingInput>,

    max_num_grid_nodes: NonZeroU32,

    steps_per_frame: u32,
    recorded_steps: u32,

    frame_input: FrameInput,

    // if we need to redo the frame
    io_particles: squishy_volumes_file_frame::Particles,

    profile_data_csv_writer: Option<ProfileDataCsvWriter>,
}

pub const BYTES_PER_GRID_NODE: u64 = 300;

impl GpuState {
    pub fn new(
        frame: usize,
        io_state: IoState,
        input_reader: squishy_volumes_file_input::InputReader,
        gpu: String,
        harness: &Harness,
        max_time_step: f32,
        profiling_output_file: Option<PathBuf>,
    ) -> Result<Self, GpuError> {
        tracing::info!("setting up GPU state");

        let harness = harness.scope("Setting up GPU State".to_string(), 5.try_into().unwrap())?;
        let time = io_state.time;
        let animated_globals = io_state.animated_globals;

        let frame_input = FrameInput::new(
            input_reader,
            io_state.goal_positions,
            io_state.collider,
            frame,
        )?;
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
        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: num_particles as u32,
        });

        let particle_goals_start = frame_input
            .goal_positions_start()
            .iter()
            .map(|p| p.push(0.))
            .collect::<Vec<_>>();
        let particle_goals_end = frame_input
            .goal_positions_end()
            .iter()
            .map(|p| p.push(0.))
            .collect::<Vec<_>>();
        harness.check()?;
        harness.step()?;

        tracing::info!("creating particle allocations");

        let start_time =
            (io_state.time % (1. / frame_input.consts().frames_per_second as f64)) as f32;
        let io_particles = io_state.particles;
        let step_input = {
            let time = Allocation::new(device, "time", &[start_time])?;
            let step = Allocation::new(device, "step", &[0])?;

            let globals = Allocation::new(device, "globals", &[animated_globals])?;

            let indirect_particles = Allocation::new(device, "indirect_particles", &[indirect])?;
            let indirect_grid_nodes =
                Allocation::new(device, "indirect_grid_nodes", &[Indirect::default()])?;

            let particle_goals_start =
                Allocation::new(device, "particle_goals_start", &particle_goals_start)?;
            let particle_goals_end =
                Allocation::new(device, "particle_goals_end", &particle_goals_end)?;

            let variable_particle_input = get_variable_particle_input(device, &io_particles)?;

            let collider_input = get_collider_input(device, &frame_input)?;

            let limits_over_time = Allocation::new(
                device,
                "limits_over_time",
                &vec![TimeStepLimits::default(); steps_per_frame as usize],
            )?;

            step::Input {
                time,
                step,

                globals,

                indirect_particles,
                indirect_grid_nodes,

                particle_goals_start,
                particle_goals_end,

                variable_particle_input,

                collider_input,

                limits_over_time,
            }
        };

        let profile_data_csv_writer = profiling_output_file
            .map(ProfileDataCsvWriter::new)
            .transpose()?;

        harness.check()?;
        harness.step()?;

        Ok(Self {
            time,
            animated_globals,
            gpu_context,
            update_flags,
            pipeline_part,
            step_input,
            pending_input: None,
            max_num_grid_nodes,
            recorded_steps: 0,
            steps_per_frame,
            frame_input,
            io_particles,
            profile_data_csv_writer,
        })
    }
}

fn get_variable_particle_input(
    device: &wgpu::Device,
    io_particles: &squishy_volumes_file_frame::Particles,
) -> Result<step::VariableParticleInput, GpuError> {
    tracing::info!("preparing variable particle data for transfer");
    let particle_positions_and_collider_bits: Vec<PositionAndColliderBits> = io_particles
        .positions
        .iter()
        .zip(&io_particles.collider_bits)
        .map(|(&position, &collider_bits)| PositionAndColliderBits {
            position: position.into(),
            collider_bits,
        })
        .collect();
    #[allow(clippy::toplevel_ref_arg)]
    let particle_position_gradients: Vec<Matrix4x3<f32>> =
        bytemuck::cast_slice::<_, Matrix3<f32>>(&io_particles.position_gradients)
            .iter()
            .map(|m| stack![m; Matrix1x3::zeros()])
            .collect();
    let particle_velocities: Vec<Vector4<f32>> =
        bytemuck::cast_slice::<_, Vector3<f32>>(&io_particles.velocities)
            .iter()
            .map(|v| v.push(0.))
            .collect();
    #[allow(clippy::toplevel_ref_arg)]
    let particle_velocity_gradients: Vec<Matrix4x3<f32>> =
        bytemuck::cast_slice::<_, Matrix3<f32>>(&io_particles.velocity_gradients)
            .iter()
            .map(|m| stack![m; Matrix1x3::zeros()])
            .collect();

    VariableParticleInput::new(
        device,
        VariableParticleInputData {
            particle_flags: &io_particles.flags,
            particle_parameters: &io_particles.parameters,
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

    let topology = frame_input.topology();
    let num_triangles = topology.triangle_indices().len();
    let num_vertices = topology.vertex_triangle_lists().len();
    tracing::info!(num_vertices, num_triangles, "preparing mesh for transfer");

    let vertex_positions_start: Vec<Vector4<f32>> = frame_input
        .collider_start()
        .vertex_positions
        .iter()
        .map(|p| p.push(0.))
        .collect();
    let vertex_positions_end: Vec<Vector4<f32>> = frame_input
        .collider_end()
        .vertex_positions
        .iter()
        .map(|p| p.push(0.))
        .collect();
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

    let triangle_frictions = Allocation::new(
        device,
        "triangle_frictions",
        &frame_input.collider_start().triangle_frictions,
    )?;
    let triangle_dampings = Allocation::new(
        device,
        "triangle_dampings",
        &frame_input.collider_start().triangle_dampings,
    )?;

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
    pub adaptive_time_steps: bool,
    pub store_bvh: bool,
    pub store_grid: bool,
}

impl GpuState {
    pub fn produce_next_state(
        &mut self,
        harness: &squishy_volumes_xpu::Harness,
        gpu_run_parameters @ GpuRunParameters {
            adaptive_time_steps,
            store_bvh,
            store_grid,
        }: GpuRunParameters,
    ) -> Result<(squishy_volumes_file_frame::IoState, Result<(), GpuError>), GpuError> {
        squishy_volumes_util::profile!("produce_next_state");

        let frame_factor = self.frame_input.frame_factor(self.time)?;
        let start_time = frame_factor / self.frame_input.consts().frames_per_second as f32;
        tracing::info!(start_time);
        self.step_input.time = Allocation::new(self.gpu_context.device(), "time", &[start_time])?;
        self.step_input.step = Allocation::new(self.gpu_context.device(), "step", &[0])?;

        let mut encoder = self
            .gpu_context
            .device()
            .create_command_encoder(&Default::default());

        let mut profiler =
            wgpu_profiler::GpuProfiler::new(self.gpu_context.device(), Default::default()).unwrap();

        let mut redo_frame = false;
        let mut buffered_error = Ok(());
        let mut mapped_downloads;

        self.recorded_steps = 0;
        if let Some(profile_data_csv_writer) = self.profile_data_csv_writer.as_mut() {
            profile_data_csv_writer.clear();
        }

        let pending_input = loop {
            let (output, new_recorded_steps) =
                self.record_steps(harness, adaptive_time_steps, &mut encoder, &profiler)?;

            profiler.resolve_queries(&mut encoder);

            let pending_input = if let Some(prepared) = self.pending_input.take() {
                prepared
            } else {
                self.prepare_pending_input()?
            };
            self.record_pending_input(&mut encoder, &pending_input)?;

            let downloads = Downloads::new(self, store_grid, output);
            downloads.copy(&mut encoder);

            tracing::info!("submit final");
            let mut tmp = self
                .gpu_context
                .device()
                .create_command_encoder(&Default::default());
            swap(&mut encoder, &mut tmp);
            self.gpu_context.queue().submit([tmp.finish()]);

            profiler.end_frame().unwrap();

            let downloads_ready = downloads.prep();

            self.wait_for_gpu(harness)?;

            tracing::info!("download");

            mapped_downloads = downloads_ready.into_mapped()?;

            let num_grid_nodes = mapped_downloads.indirect_nodes.len;
            tracing::info!(self.max_num_grid_nodes, num_grid_nodes);

            if let Some(profile_data_csv_writer) = self.profile_data_csv_writer.as_mut() {
                profile_data_csv_writer.buffer_data(
                    &self.gpu_context,
                    &mut profiler,
                    new_recorded_steps,
                )?;
            }

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
                        self.step_input.limits_over_time.buffer(),
                        self.step_input.limits_over_time.offset(),
                        limits_over_time.buffer(),
                        limits_over_time.offset(),
                        Some(self.step_input.limits_over_time.size().get()),
                    );
                    self.step_input.limits_over_time = limits_over_time;
                    self.pending_input = Some(pending_input);
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
                }
                Err(GpuError::Shader(GpuShaderError::FrameTimeReached)) => {}
                x => x?,
            };
            break pending_input;
        };

        if redo_frame {
            if self.max_num_grid_nodes.get() as usize >= self.io_particles.flags.len() * 27 {
                return Err(GpuError::MaxGridNodesExceeded);
            }

            self.max_num_grid_nodes = (self.max_num_grid_nodes.get() * 2).try_into().unwrap();
            tracing::warn!(self.max_num_grid_nodes, "The frame needs to be redone");

            self.step_input.variable_particle_input =
                get_variable_particle_input(self.gpu_context.device(), &self.io_particles)?;
            self.gpu_context.resize_allocator(
                self.max_num_grid_nodes.get() as u64 * BYTES_PER_GRID_NODE,
                false,
            )?;
            self.pending_input = Some(pending_input);
            return self.produce_next_state(harness, gpu_run_parameters);
        }

        let MappedDownloads {
            status: _,
            time,
            //step: _,
            //limits_over_time: _,
            indirect_nodes,
            particle_flags,
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            grid,
        } = mapped_downloads;

        let advanced_time = time - start_time;
        self.time += advanced_time as f64;
        self.animated_globals = pending_input.animated_globals;
        self.step_input.globals = pending_input.globals;
        self.step_input.particle_goals_start = self.step_input.particle_goals_end.clone();
        self.step_input.particle_goals_end = pending_input.next_goal_positions;
        self.step_input.collider_input = pending_input.next_collider_input;

        if let Some(profile_data_csv_writer) = self.profile_data_csv_writer.as_mut() {
            profile_data_csv_writer.write_frame(
                self.time..self.time + 1. / self.frame_input.consts().frames_per_second as f64,
            )?;
        };

        let particles = {
            let flags = particle_flags;
            // TODO: this might be change soon
            let parameters = self.io_particles.parameters.clone();
            // TODO
            let elastic_energies = vec![0.; flags.len()];
            let (positions, collider_bits) = particle_positions_and_collider_bits
                .into_iter()
                .map(
                    |PositionAndColliderBits {
                         position,
                         collider_bits,
                     }|
                     -> ([f32; 3], u32) { (position.into(), collider_bits) },
                )
                .unzip();
            let position_gradients = particle_position_gradients
                .into_iter()
                .map(|m| m.fixed_view::<3, 3>(0, 0).into())
                .collect();
            let velocities = particle_velocities
                .into_iter()
                .map(|v| v.xyz().into())
                .collect();
            // https://github.com/Algebraic-UG/squishy_volumes/issues/368
            let velocity_gradients = self.io_particles.velocity_gradients.clone();

            // TODO: does it make sense to have this "variable"
            let initial_positions = self.io_particles.initial_positions.clone();

            squishy_volumes_file_frame::Particles {
                flags,
                parameters,
                elastic_energies,
                collider_bits,
                positions,
                position_gradients,
                velocities,
                velocity_gradients,
                initial_positions,
            }
        };

        let collider = self.frame_input.collider_start().to_io_collider();
        let goal_positions = bytemuck::cast_slice(self.frame_input.goal_positions_end()).to_vec();
        let bvh = store_bvh.then(|| self.frame_input.bvh().clone());
        let grid_nodes = grid.map(
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
        let io_state = IoState {
            time: self.time,
            animated_globals: self.animated_globals,
            particles,
            goal_positions,
            collider,
            bvh,
            grid_nodes,
        };

        Ok((io_state, buffered_error))
    }

    fn prepare_pending_input(&mut self) -> Result<PendingInput, GpuError> {
        let Some(next_input_frame) = self.frame_input.next_input_frame() else {
            return Ok(PendingInput {
                animated_globals: self.animated_globals,
                globals: self.step_input.globals.clone(),
                next_goal_positions: self.step_input.particle_goals_end.clone(),
                next_collider_input: self.step_input.collider_input.clone(),
                particle_updates: Vec::new(),
            });
        };

        let globals = Allocation::new(
            self.gpu_context.device(),
            "globals",
            &[next_input_frame.animated_globals],
        )?;

        let input_ranges = self.frame_input.input_ranges();
        let mut particle_updates = Vec::new();
        for bulk in next_input_frame.bulk {
            if let BulkAttribute::Particles(attribute) = bulk.meta.captured_attribute {
                let InputRangeParticles { particle_range } = input_ranges
                    .get_particle_range(&bulk.meta.object_name)
                    .map_err(|error| error.attach_frame(self.frame_input.frame() + 1))
                    .map_err(FrameInputError::InputError)?;

                let offset = particle_range.start as u32;
                let allocation = match attribute {
                    FrameBulkParticles::Flags => {
                        let flags: &[ParticleFlags] = bulk
                            .data
                            .assume_ints()
                            .map_err(|error| {
                                error
                                    .attach_attr(bulk.meta.captured_attribute)
                                    .attach_name(bulk.meta.object_name.clone())
                                    .attach_frame(self.frame_input.frame() + 1)
                            })
                            .map_err(FrameInputError::InputError)?;

                        Allocation::new(self.gpu_context.device(), "new_flags", flags)?
                    }
                    FrameBulkParticles::ColliderBits => todo!(),
                    FrameBulkParticles::Transforms => todo!(),
                    FrameBulkParticles::Sizes => todo!(),
                    FrameBulkParticles::Densities => todo!(),
                    FrameBulkParticles::YoungsModuluses => todo!(),
                    FrameBulkParticles::PoissonsRatios => todo!(),
                    FrameBulkParticles::InitialPositions => todo!(),
                    FrameBulkParticles::InitialVelocity => todo!(),
                    FrameBulkParticles::ViscosityDynamic => todo!(),
                    FrameBulkParticles::ViscosityBulk => todo!(),
                    FrameBulkParticles::Exponent => todo!(),
                    FrameBulkParticles::BulkModulus => todo!(),
                    FrameBulkParticles::SandAlpha => todo!(),

                    // already handled
                    FrameBulkParticles::GoalPositions => {
                        continue;
                    }
                };
                particle_updates.push(PendingParticleUpdate {
                    attribute,
                    allocation,
                    offset,
                });
            }
        }

        self.frame_input.load_next()?;

        let next_goal_positions = Allocation::new(
            self.gpu_context.device(),
            "particle_goals_end",
            &self
                .frame_input
                .goal_positions_end()
                .iter()
                .map(|p| p.push(0.))
                .collect::<Vec<_>>(),
        )?;

        let next_collider_input = get_collider_input(self.gpu_context.device(), &self.frame_input)?;

        Ok(PendingInput {
            animated_globals: next_input_frame.animated_globals,
            globals,
            next_goal_positions,
            next_collider_input,
            particle_updates,
        })
    }

    fn record_pending_input(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        pending_input: &PendingInput,
    ) -> Result<(), GpuError> {
        for PendingParticleUpdate {
            attribute,
            allocation,
            offset,
        } in &pending_input.particle_updates
        {
            match attribute {
                FrameBulkParticles::Flags => {
                    self.update_flags.record(
                        &mut self.gpu_context,
                        &mut encoder.into(),
                        update_flags::Input {
                            new_flags: allocation.clone(),
                            flags: self
                                .step_input
                                .variable_particle_input
                                .particle_flags
                                .clone(),
                        },
                        update_flags::Parameters { offset: *offset },
                    )?;
                }

                FrameBulkParticles::ColliderBits => todo!(),
                FrameBulkParticles::Transforms => todo!(),
                FrameBulkParticles::Sizes => todo!(),
                FrameBulkParticles::Densities => todo!(),
                FrameBulkParticles::YoungsModuluses => todo!(),
                FrameBulkParticles::PoissonsRatios => todo!(),
                FrameBulkParticles::InitialPositions => todo!(),
                FrameBulkParticles::InitialVelocity => todo!(),
                FrameBulkParticles::ViscosityDynamic => todo!(),
                FrameBulkParticles::ViscosityBulk => todo!(),
                FrameBulkParticles::Exponent => todo!(),
                FrameBulkParticles::BulkModulus => todo!(),
                FrameBulkParticles::SandAlpha => todo!(),

                FrameBulkParticles::GoalPositions => unreachable!(),
            }
        }
        Ok(())
    }

    fn record_steps(
        &mut self,
        harness: &Harness,
        adaptive_time_steps: bool,
        encoder: &mut wgpu::CommandEncoder,
        profiler: &wgpu_profiler::GpuProfiler,
    ) -> Result<(step::Output, usize), GpuError> {
        tracing::info!(assumed_steps = self.steps_per_frame, "Recording steps");
        let mut new_recorded_steps = 0;
        loop {
            harness.check()?;
            let scope = profiler.scope("run_step", encoder);
            let output = self.pipeline_part.record(
                &mut self.gpu_context,
                &mut scope.into(),
                self.step_input.clone(),
                step::Parameters {
                    max_num_grid_nodes: self.max_num_grid_nodes,
                    current_step: self.recorded_steps,
                    adaptive_time_steps,
                },
            )?;
            self.recorded_steps += 1;
            new_recorded_steps += 1;

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
                break Ok((output, new_recorded_steps));
            }
        }
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
                gpu_state.step_input.time.clone(),
                //gpu_state.next_input.step.clone(),
                //gpu_state.next_input.limits_over_time.clone(),
                gpu_state.step_input.indirect_grid_nodes.clone(),
                gpu_state
                    .step_input
                    .variable_particle_input
                    .particle_flags
                    .clone(),
                gpu_state
                    .step_input
                    .variable_particle_input
                    .particle_positions_and_collider_bits
                    .clone(),
                gpu_state
                    .step_input
                    .variable_particle_input
                    .particle_position_gradients
                    .clone(),
                gpu_state
                    .step_input
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
            time,
            //step,
            //limits_over_time,
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
            //limits_over_time: limits_over_time.to_vec()?,
            time: time.to_vec()?[0],
            //step: step.to_vec()?[0],
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
    time: f32,
    // TODO: load this for stats
    //step: u32,
    //limits_over_time: Vec<TimeStepLimits>,
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
