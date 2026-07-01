// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Result, bail};
use nalgebra::{Matrix1x3, Matrix4x3, Vector4, stack};
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;
use squishy_volumes_gpu::{
    Allocation, BoundingVolumeHierarchyAllocations, DispatchSettings, GpuAllocatorError, Indirect,
    PipelinePart, PositionAndColliderBits, prefix_sum_on_cpu,
};
use std::collections::BTreeMap;

use crate::{
    phase::{Phase, PhaseInput},
    profile,
    stats::StateStats,
};

pub mod attributes;
pub mod grids;
pub mod initialization;
mod interpolated_input;
pub mod object;
pub mod particles;
pub mod util;

use grids::GridMomentum;
pub use interpolated_input::InterpolatedInput;
use object::{ObjectCollider, ObjectParticles};
use particles::Particles;

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct State {
    pub time: f64,
    pub phase: Phase,

    pub name_map: BTreeMap<String, ObjectIndex>,

    pub particle_objects: Vec<ObjectParticles>,
    pub collider_objects: Vec<ObjectCollider>,

    pub particles: Particles,
    pub grid: GridMomentum,

    #[serde(skip)]
    pub interpolated_input: Option<InterpolatedInput>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ObjectIndex {
    Particles(usize),
    Collider(usize),
}

impl State {
    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn frame_factor(&self, phase_input: &PhaseInput) -> Result<T> {
        let frame_time = self.time * phase_input.consts.frames_per_second as f64;
        assert!(phase_input.next_frame != 0);

        if frame_time < (phase_input.next_frame - 1) as f64
            || frame_time > phase_input.next_frame as f64
        {
            bail!(
                "Mismatch between frame_time {frame_time} and next_frame {}",
                phase_input.next_frame
            );
        }

        Ok((frame_time % 1.) as T)
    }

    pub fn stats(&self) -> StateStats {
        let total_particle_count = self.particles.reverse_sort_map.len();
        let total_grid_node_count = self.grid.map.len();
        let per_object_count = self
            .name_map
            .iter()
            .map(|(name, object_idx)| {
                (
                    name.clone(),
                    match object_idx {
                        ObjectIndex::Particles(idx) => self.particle_objects[*idx].particles.len(),
                        ObjectIndex::Collider(_) => 0,
                    },
                )
            })
            .collect();

        StateStats {
            total_particle_count,
            total_grid_node_count,
            per_object_count,
        }
    }

    pub fn to_gpu_state(
        &self,
        phase_input: &mut PhaseInput,
        mut gpu_context: squishy_volumes_gpu::GpuContext,
    ) -> Result<GpuState, GpuAllocatorError> {
        profile!("to_gpu_state");

        tracing::info!("creating GPU state");

        let workgroup_size = 64.try_into().unwrap();
        let dispatch_limit = gpu_context
            .device()
            .limits()
            .max_compute_workgroups_per_dimension
            .try_into()
            .unwrap();

        tracing::info!(workgroup_size, dispatch_limit);

        tracing::info!("creating pipeline");
        let pipeline_part = squishy_volumes_gpu::Step::new(
            &mut gpu_context,
            squishy_volumes_gpu::step::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size: phase_input.consts.scaled_grid_node_size(),
                forget_distance: phase_input.consts.forget_distance(),
                accept_distance: phase_input.consts.accept_distance(),
                time_step: phase_input.time_step,
                table_tries: 50,
            },
        );

        let device = gpu_context.device();

        let num_particles = self.particles.sort_map.len();
        tracing::info!(num_particles, "preparing particles for transfer");
        let particle_parameters: Vec<squishy_volumes_gpu::particle_parameters::Device> = self
            .particles
            .parameters
            .iter()
            .map(|parameter| {
                match parameter.clone() {
                    crate::state::particles::ParticleParameters::Solid {
                        mu,
                        lambda,
                        viscosity: _,
                        sand_alpha,
                    } => squishy_volumes_gpu::particle_parameters::Host::Solid(
                        squishy_volumes_gpu::particle_parameters::Solid {
                            mu,
                            lambda,
                            viscosity: None,
                            sand_alpha,
                        },
                    ),
                    crate::state::particles::ParticleParameters::Fluid {
                        exponent,
                        bulk_modulus,
                        viscosity: _,
                    } => squishy_volumes_gpu::particle_parameters::Host::Fluid(
                        squishy_volumes_gpu::particle_parameters::Fluid {
                            exponent,
                            bulk_modulus,
                            viscosity: None,
                        },
                    ),
                }
                .into()
            })
            .collect();
        let particle_positions_and_collider_bits: Vec<PositionAndColliderBits> = self
            .particles
            .positions
            .iter()
            .zip(&self.particles.collider_bits)
            .map(|(&position, &collider_bits)| PositionAndColliderBits {
                position,
                collider_bits,
            })
            .collect();
        #[allow(clippy::toplevel_ref_arg)]
        let particle_position_gradients: Vec<Matrix4x3<f32>> = self
            .particles
            .position_gradients
            .iter()
            .map(|m| stack![m; Matrix1x3::zeros()])
            .collect();
        let particle_velocities: Vec<Vector4<f32>> = self
            .particles
            .velocities
            .iter()
            .map(|v| v.push(0.))
            .collect();
        #[allow(clippy::toplevel_ref_arg)]
        let particle_velocity_gradients: Vec<Matrix4x3<f32>> = self
            .particles
            .velocity_gradients
            .iter()
            .map(|m| stack![m; Matrix1x3::zeros()])
            .collect();

        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: num_particles as u32,
        });

        let a = phase_input.input_interpolation.a();
        let b = phase_input.input_interpolation.b().unwrap_or(a);

        tracing::info!("creating particle allocations");

        // TODO: interpolate that
        let gravity = Allocation::new(device, "gravity", &[a.gravity().push(0.)])?;

        let indirect_particles = Allocation::new(device, "indirect_particles", &[indirect])?;
        let particle_masses = Allocation::new(device, "particle_masses", &self.particles.masses)?;
        let particle_initial_volumes = Allocation::new(
            device,
            "particle_initial_volumes",
            &self.particles.initial_volumes,
        )?;
        let particle_parameters =
            Allocation::new(device, "particle_parameters", &particle_parameters)?;
        let particle_positions_and_collider_bits = Allocation::new(
            device,
            "particle_positions_and_collider_bits",
            &particle_positions_and_collider_bits,
        )?;
        let particle_position_gradients = Allocation::new(
            device,
            "particle_position_gradients",
            &particle_position_gradients,
        )?;
        let particle_velocities =
            Allocation::new(device, "particle_velocities", &particle_velocities)?;
        let particle_velocity_gradients = Allocation::new(
            device,
            "particle_velocity_gradients",
            &particle_velocity_gradients,
        )?;

        let collider_input = (!phase_input.input_interpolation.topology().is_empty())
            .then(|| {
                let topology = phase_input.input_interpolation.topology();
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
                let triangle_frictions =
                    Allocation::new(device, "triangle_frictions", a.triangle_frictions())?;

                let num_bvh_levels = phase_input.input_interpolation.bvh().level();
                let num_bvh_nodes = phase_input.input_interpolation.bvh().nodes().len();
                tracing::info!(num_bvh_levels, num_bvh_nodes, "creating bvh allocations");
                let bvh = BoundingVolumeHierarchyAllocations::new(
                    device,
                    phase_input.consts.leaf_size,
                    phase_input.input_interpolation.bvh(),
                )?;

                Ok(squishy_volumes_gpu::step::ColliderInput {
                    vertex_positions_start,
                    vertex_positions_end,
                    vertex_triangle_offsets,
                    vertex_triangle_lists,
                    triangle_indices,
                    triangle_collider,
                    triangle_opposites,
                    triangle_frictions,
                    bvh,
                })
            })
            .transpose()?;

        let next_input = squishy_volumes_gpu::step::Input {
            gravity,
            indirect_particles,
            particle_masses,
            particle_initial_volumes,
            particle_parameters,
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,

            collider_input,
        };

        Ok(GpuState {
            gpu_context,
            pipeline_part,
            next_input,
        })
    }
}

pub struct GpuState {
    pub gpu_context: squishy_volumes_gpu::GpuContext,
    pub pipeline_part: squishy_volumes_gpu::Step,
    pub next_input: squishy_volumes_gpu::step::Input,
}
