// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU32;

use nalgebra::{Matrix1x3, Matrix3, Matrix4x3, Vector3, Vector4, stack};
use squishy_volumes_file_frame::IoState;
use squishy_volumes_xpu::FrameInput;

use crate::{
    particle_parameters::ParticleParametersDevice,
    step::{VariableParticleInput, VariableParticleInputData},
};

use super::*;

pub struct GpuState {
    time: f64,
    gpu_context: GpuContext,
    pipeline_part: Step,
    next_input: step::Input,
    max_num_grid_nodes: NonZeroU32,
    previous_io_state: IoState,
}

pub const BYTES_PER_GRID_NODE: u64 = 300;

impl GpuState {
    pub fn from_io_state(
        frame_input: &FrameInput,
        time_step: f32,
        io_state: IoState,
    ) -> Result<Self, GpuError> {
        tracing::info!("setting up GPU state");

        let time = io_state.time;
        let consts = frame_input.consts();

        let max_num_grid_nodes: NonZeroU32 = if let Some(grid_nodes) = io_state.grid_nodes.as_ref()
        {
            (grid_nodes.collider_bits.len() as f64 * 1.1) as u32
        } else {
            io_state.particles.flags.len() as u32
        }
        .max(1000)
        .try_into()
        .unwrap();

        tracing::info!(max_num_grid_nodes, "this is the limit for now");

        let mut gpu_context = GpuContext::new()?;

        tracing::info!("setting up GPU allocators");
        gpu_context.setup_allocator(
            max_num_grid_nodes.get() as u64 * BYTES_PER_GRID_NODE,
            "main allocator",
            false,
        )?;
        gpu_context.setup_indirect_allocator(2048, "indirect allocator", false)?;

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
        );

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

        Ok(Self {
            time,
            gpu_context,
            pipeline_part,
            next_input,
            max_num_grid_nodes,
            previous_io_state: io_state,
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
