// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{num::NonZeroU32, time::Duration};

use nalgebra::{Matrix1x3, Matrix3, Matrix4x3, Vector3, Vector4, stack};
use squishy_volumes_file_frame::IoState;
use squishy_volumes_xpu::{FrameInput, Harness};

use crate::{
    particle_parameters::ParticleParametersDevice,
    step::{VariableParticleInput, VariableParticleInputData},
};

use super::*;

pub struct GpuState {
    time: f64,
    time_step: f32,
    gpu_context: GpuContext,
    pipeline_part: Step,
    next_input: step::Input,
    max_num_grid_nodes: NonZeroU32,
    io_state: IoState,
    profile_data_csv_writer: ProfileDataCsvWriter,
}

pub const BYTES_PER_GRID_NODE: u64 = 300;

impl GpuState {
    pub fn from_io_state(
        harness: &Harness,
        frame_input: &FrameInput,
        time_step: f32,
        io_state: IoState,
    ) -> Result<Self, GpuError> {
        tracing::info!("setting up GPU state");
        let harness = harness.scope("Setting up GPU State".to_string(), 5.try_into().unwrap())?;

        let time = io_state.time;
        let consts = frame_input.consts();

        let max_num_grid_nodes: NonZeroU32 = (io_state.particles.flags.len() as u32)
            .max(1000)
            .try_into()
            .unwrap();

        tracing::info!(max_num_grid_nodes, "this is the limit for now");

        let mut gpu_context = GpuContext::new()?;
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
        let pipeline_part = Step::new(
            &mut gpu_context,
            step::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size: consts.scaled_grid_node_size(),
                forget_distance: consts.forget_distance(),
                accept_distance: consts.accept_distance(),
                time_step,
                table_tries: 50, // TODO: make configurable?
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

        // TODO: interpolate that
        let gravity = Allocation::new(device, "gravity", &[a.gravity().push(0.)])?;

        let indirect_particles = Allocation::new(device, "indirect_particles", &[indirect])?;
        let particle_parameters =
            Allocation::new(device, "particle_parameters", &particle_parameters)?;

        let particle_goals_start =
            Allocation::new(device, "particle_goals_start", &particle_goals_start)?;
        let particle_goals_end =
            Allocation::new(device, "particle_goals_end", &particle_goals_end)?;

        let variable_particle_input = get_variable_particle_input(device, &io_state)?;

        let collider_input = get_collider_input(device, frame_input)?;

        let next_input = step::Input {
            gravity,
            indirect_particles,

            particle_parameters,

            particle_goals_start,
            particle_goals_end,

            variable_particle_input,

            collider_input,
        };

        // TODO: make configurable
        let profile_data_csv_writer = ProfileDataCsvWriter::new("profile.csv")?;

        harness.check()?;
        harness.step()?;

        Ok(Self {
            time,
            time_step,
            gpu_context,
            pipeline_part,
            next_input,
            max_num_grid_nodes,
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

    let vertex_triangle_lists = topology.vertex_triangle_lists();
    let vertex_triangle_offsets = prefix_sum_on_cpu(
        &vertex_triangle_lists
            .iter()
            .map(|v| v.len() as u32)
            .collect::<Vec<_>>(),
    );
    let mut vertex_triangle_lists: Vec<u32> = vertex_triangle_lists
        .iter()
        .flat_map(|list| list.iter().cloned())
        .collect();
    if vertex_triangle_lists.is_empty() {
        tracing::warn!("all vertices are on open edges");
        vertex_triangle_lists.push(0);
    }

    tracing::info!("creating mesh allocations");
    let vertex_positions_start =
        Allocation::new(device, "vertex_positions_start", &vertex_positions_start)?;
    let vertex_positions_end =
        Allocation::new(device, "vertex_positions_end", &vertex_positions_end)?;
    let vertex_triangle_offsets =
        Allocation::new(device, "vertex_triangle_offsets", &vertex_triangle_offsets)?;
    let vertex_triangle_lists =
        Allocation::new(device, "vertex_triangle_lists", &vertex_triangle_lists)?;

    let triangle_indices =
        Allocation::new(device, "triangle_indices", topology.triangle_indices())?;
    let triangle_collider =
        Allocation::new(device, "triangle_collider", topology.triangle_collider())?;
    let triangle_opposites =
        Allocation::new(device, "triangle_opposites", topology.triangle_opposites())?;

    // TODO: interpolate that
    let triangle_frictions = Allocation::new(device, "triangle_frictions", a.triangle_frictions())?;

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
        vertex_triangle_offsets,
        vertex_triangle_lists,
        triangle_indices,
        triangle_collider,
        triangle_opposites,
        triangle_frictions,
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
    ) -> Result<squishy_volumes_file_frame::IoState, GpuError> {
        squishy_volumes_util::profile!("produce_next_state");

        if self.time >= target_time {
            return Ok(self.io_state.clone());
        }

        let mut encoder = self
            .gpu_context
            .device()
            .create_command_encoder(&Default::default());
        let mut profiler =
            wgpu_profiler::GpuProfiler::new(self.gpu_context.device(), Default::default()).unwrap();

        let mut recorded_steps = 0;
        let output = loop {
            harness.check()?;

            let scope = profiler.scope("run_step", &mut encoder);
            let output = self.pipeline_part.record(
                &mut self.gpu_context,
                &mut scope.into(),
                self.next_input.clone(),
                step::Parameters {
                    max_num_grid_nodes: self.max_num_grid_nodes,
                    factor: frame_input.frame_factor(self.time)?,
                },
            )?;
            self.time += self.time_step as f64;

            recorded_steps += 1;
            if recorded_steps > 10 {
                tracing::info!("submit");
                self.gpu_context.queue().submit([encoder.finish()]);
                encoder = self
                    .gpu_context
                    .device()
                    .create_command_encoder(&Default::default());
                recorded_steps = 0;
            }

            if self.time >= target_time {
                break output;
            }
        };

        let downloads = DownloadsToHost::new(
            &self.gpu_context,
            [
                self.gpu_context.status(),
                output.indirect_nodes,
                self.next_input
                    .variable_particle_input
                    .particle_positions_and_collider_bits
                    .clone(),
                self.next_input
                    .variable_particle_input
                    .particle_position_gradients
                    .clone(),
                self.next_input
                    .variable_particle_input
                    .particle_velocities
                    .clone(),
            ],
        );
        let downloads_grid = store_grid.then(|| {
            DownloadsToHost::new(
                &self.gpu_context,
                [output.node_ids_and_collider_bits, output.node_momentums],
            )
        });

        downloads.copy(&mut encoder);
        if let Some(downloads_grid) = downloads_grid.as_ref() {
            downloads_grid.copy(&mut encoder);
        }

        profiler.resolve_queries(&mut encoder);

        tracing::info!("submit final");
        self.gpu_context.queue().submit([encoder.finish()]);

        let downloads_ready = downloads.prep();
        let downloads_grid_ready = downloads_grid.as_ref().map(DownloadsToHost::prep);
        profiler.end_frame().unwrap();

        tracing::info!("prepare next frame input");
        frame_input.load(frame_input.frame() + 1)?;

        let b = frame_input.b().unwrap_or(frame_input.a());

        let particle_goals_end = b
            .particle_goal_positions()
            .iter()
            .map(|p| p.push(0.))
            .collect::<Vec<_>>();

        // TODO: this will need something else for when we have culling on GPU
        self.next_input.variable_particle_input.particle_flags = Allocation::new(
            self.gpu_context.device(),
            "particle_flags",
            b.particle_flags(),
        )?;
        self.next_input.particle_goals_start = self.next_input.particle_goals_end.clone();
        self.next_input.particle_goals_end = Allocation::new(
            self.gpu_context.device(),
            "particle_goals_end",
            &particle_goals_end,
        )?;

        if let Some(collider_input) = self.next_input.collider_input.as_mut() {
            let vertex_positions_end: Vec<Vector4<f32>> =
                b.vertex_positions().iter().map(|p| p.push(0.)).collect();
            collider_input.vertex_positions_start = collider_input.vertex_positions_end.clone();
            collider_input.vertex_positions_end = Allocation::new(
                self.gpu_context.device(),
                "vertex_positions_end",
                &vertex_positions_end,
            )?;

            collider_input.bvh = BoundingVolumeHierarchyAllocations::new(
                self.gpu_context.device(),
                frame_input.consts().leaf_size,
                frame_input.bvh(),
            )?;
        }

        tracing::info!("waiting on GPU");
        loop {
            match self.gpu_context.device().poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_millis(100)),
            }) {
                Ok(_) => break,
                Err(wgpu::PollError::Timeout) => {
                    harness.check()?;
                }
                error => {
                    error?;
                }
            }
        }

        // TODO: what if the frame needs to be redone?
        self.profile_data_csv_writer.write_frame(
            &self.gpu_context,
            &mut profiler,
            frame_input.frame(),
        )?;

        tracing::info!("download");

        let [
            status,
            indirect_nodes_download,
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
        ] = downloads_ready.try_into().unwrap();

        let num_grid_nodes = indirect_nodes_download.to_vec::<Indirect>()?[0].len;
        tracing::info!(self.max_num_grid_nodes, num_grid_nodes);

        let mut redo_frame = false;
        match status.to_vec::<GpuStatus>()?[0].to_result(&self.gpu_context) {
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
        self.gpu_context.reset_status()?;

        if redo_frame {
            drop(downloads);
            drop(downloads_grid);
            self.time = self.io_state.time;
            self.max_num_grid_nodes = (self.max_num_grid_nodes.get() * 2).try_into().unwrap();
            tracing::warn!(self.max_num_grid_nodes, "The frame needs to be redone");
            frame_input.load(frame_input.frame() - 1)?;
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

        let particle_positions_and_collider_bits: Vec<PositionAndColliderBits> =
            particle_positions_and_collider_bits.to_vec()?;

        self.io_state.time = self.time;
        self.io_state.particles.collider_bits = particle_positions_and_collider_bits
            .iter()
            .map(|position_and_bits| position_and_bits.collider_bits)
            .collect();
        self.io_state.particles.positions = particle_positions_and_collider_bits
            .into_iter()
            .map(|position_and_bits| position_and_bits.position.into())
            .collect();
        self.io_state.particles.position_gradients = particle_position_gradients
            .to_vec::<Matrix4x3<f32>>()?
            .into_iter()
            .map(|m| m.fixed_view::<3, 3>(0, 0).into())
            .collect();
        self.io_state.particles.velocities = particle_velocities
            .to_vec::<Vector4<f32>>()?
            .into_iter()
            .map(|v| v.xyz().into())
            .collect();

        self.io_state.grid_nodes = downloads_grid_ready
            .map(|downloads_grid_ready| {
                let [node_ids_and_collider_bits, node_momentums] =
                    downloads_grid_ready.try_into().unwrap();
                let node_ids_and_collider_bits: Vec<NodeIdAndColliderBits> =
                    node_ids_and_collider_bits.to_vec()?;
                let node_momentums: Vec<Vector4<f32>> = node_momentums.to_vec()?;

                let node_ids = node_ids_and_collider_bits
                    .iter()
                    .take(num_grid_nodes as usize)
                    .map(|node_id_and_collider_bits| node_id_and_collider_bits.node_id.into())
                    .collect();
                let collider_bits = node_ids_and_collider_bits
                    .iter()
                    .take(num_grid_nodes as usize)
                    .map(|node_id_and_collider_bits| node_id_and_collider_bits.collider_bits)
                    .collect();
                let masses = node_momentums
                    .iter()
                    .take(num_grid_nodes as usize)
                    .map(|momentum| momentum.w)
                    .collect();
                let velocites = node_momentums
                    .iter()
                    .take(num_grid_nodes as usize)
                    .map(|momentum| {
                        if momentum.w != 0. {
                            momentum.xyz() / momentum.w
                        } else {
                            Vector3::zeros()
                        }
                        .into()
                    })
                    .collect();

                Ok::<_, GpuError>(squishy_volumes_file_frame::GridNodes {
                    node_ids,
                    collider_bits,
                    masses,
                    velocites,
                })
            })
            .transpose()?;

        Ok(self.io_state.clone())
    }
}
