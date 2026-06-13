// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix1x3, Matrix4x3, Vector4, stack};
use serde::{Deserialize, Serialize};
use squishy_volumes_gpu::PipelinePart as _;
use squishy_volumes_util::{collider_bits, mesh::compute_triangle_opposites, triangle::Triangle};
use std::{
    collections::BTreeMap,
    iter::{once, repeat},
};

use crate::{
    input_interpolation::InterpolatedInput,
    phase::{Phase, PhaseInput},
    profile,
    state::grids::GridCollider,
    stats::StateStats,
};

pub mod attributes;
pub mod grids;
pub mod initialization;
pub mod object;
pub mod particles;
pub mod util;

use grids::GridMomentum;
use object::{ObjectCollider, ObjectParticles};
use particles::Particles;

#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    pub time: f64,
    pub phase: Phase,

    pub name_map: BTreeMap<String, ObjectIndex>,

    pub particle_objects: Vec<ObjectParticles>,
    pub collider_objects: Vec<ObjectCollider>,

    pub particles: Particles,
    pub grid_momentum: GridMomentum,
    pub grid_collider: GridCollider,

    pub grid_collider_momentums: Vec<GridMomentum>,

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

    pub fn grid_momentums(&self) -> impl Iterator<Item = &GridMomentum> {
        once(&self.grid_momentum).chain(self.grid_collider_momentums.iter())
    }

    pub fn grid_momentums_mut(&mut self) -> impl Iterator<Item = &mut GridMomentum> {
        once(&mut self.grid_momentum).chain(self.grid_collider_momentums.iter_mut())
    }

    pub fn stats(&self) -> StateStats {
        let total_particle_count = self.particles.reverse_sort_map.len();
        let total_grid_node_count = self.grid_momentums().map(|grid| grid.masses.len()).sum();
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
        gpu_context: squishy_volumes_gpu::GpuContext,
    ) -> GpuState {
        profile!("to_gpu_state");
        let time_step = phase_input.time_step;
        let grid_node_size = phase_input.consts.scaled_grid_node_size();
        let cell_size = grid_node_size * 2.;
        let settings = squishy_volumes_gpu::step::Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            cell_size,
            forget_distance: cell_size * 1.1,
            accept_distance: cell_size,
            bit_count: 2.try_into().unwrap(),
            time_step,
        };

        let indices = self
            .particles
            .sort_map
            .iter()
            .map(|index| *index as u32)
            .collect::<Vec<_>>();
        let parameters: Vec<squishy_volumes_gpu::particle_parameters::Device> = self
            .particles
            .parameters
            .iter()
            .map(|parameter| {
                match parameter.clone() {
                    crate::state::particles::ParticleParameters::Solid {
                        mu,
                        lambda,
                        viscosity: _,
                        sand_alpha: _,
                    } => squishy_volumes_gpu::particle_parameters::Host::Solid(
                        squishy_volumes_gpu::particle_parameters::Solid {
                            mu,
                            lambda,
                            viscosity: None,
                            sand_alpha: None,
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
        let positions: Vec<Vector4<f32>> = self
            .particles
            .positions
            .iter()
            .map(|p| p.push(0.))
            .collect();
        #[allow(clippy::toplevel_ref_arg)]
        let position_gradients: Vec<Matrix4x3<f32>> = self
            .particles
            .position_gradients
            .iter()
            .map(|m| stack![m; Matrix1x3::zeros()])
            .collect();
        let velocities: Vec<Vector4<f32>> = self
            .particles
            .velocities
            .iter()
            .map(|v| v.push(0.))
            .collect();
        #[allow(clippy::toplevel_ref_arg)]
        let velocity_gradients: Vec<Matrix4x3<f32>> = self
            .particles
            .velocity_gradients
            .iter()
            .map(|m| stack![m; Matrix1x3::zeros()])
            .collect();

        let collider_bits: Vec<u32> = self
            .particles
            .collider_insides
            .iter()
            .map(|collider_insides| {
                let mut bits = 0;
                for collider in 0..16 {
                    collider_bits::set(
                        &mut bits,
                        collider,
                        collider_insides.get(&collider).cloned(),
                    );
                }
                bits
            })
            .collect();

        let a = phase_input
            .input_interpolation
            .a()
            .map(|point| &point.interpolant)
            .expect("there's always the a point");
        let b = phase_input
            .input_interpolation
            .b()
            .map(|point| &point.interpolant)
            .unwrap_or(a);

        let vertex_positions_start: Vec<Vector4<f32>> = a
            .collider_input
            .values()
            .flat_map(|collider_input| collider_input.vertex_positions.iter().map(|p| p.push(0.)))
            .collect();
        let vertex_positions_end: Vec<Vector4<f32>> = b
            .collider_input
            .values()
            .flat_map(|collider_input| collider_input.vertex_positions.iter().map(|p| p.push(0.)))
            .collect();
        let triangle_indices: Vec<Triangle> = a
            .collider_input
            .values()
            .flat_map(|collider_input| {
                collider_input
                    .triangles
                    .iter()
                    .map(|t| t.iter().cloned().into())
            })
            .collect();
        let triangle_collider: Vec<u32> = a
            .collider_input
            .iter()
            .flat_map(|(name, collider_input)| {
                let ObjectIndex::Collider(collider) = self.name_map[name] else {
                    panic!("object type mismatch {name}");
                };
                repeat(collider as u32).take(collider_input.triangles.len())
            })
            .collect();
        let triangle_opposites = compute_triangle_opposites(&triangle_indices);
        let triangle_frictions: Vec<f32> = a
            .collider_input
            .values()
            .flat_map(|collider_input| collider_input.triangle_frictions.iter().cloned())
            .collect();

        let next_input = squishy_volumes_gpu::step::Input::new(
            gpu_context.device(),
            cell_size, //leaf_size
            16,        //leaf_threshold
            settings.clone(),
            squishy_volumes_gpu::step::InputData {
                indices: &indices,
                collider_bits: &collider_bits,
                masses: &self.particles.masses,
                initial_volumes: &self.particles.initial_volumes,
                parameters: &parameters,
                positions: &positions,
                position_gradients: &position_gradients,
                velocities: &velocities,
                velocity_gradients: &velocity_gradients,

                vertex_positions_start: &vertex_positions_start,
                vertex_positions_end: &vertex_positions_end,
                triangle_indices: &triangle_indices,
                triangle_collider: &triangle_collider,
                triangle_opposites: &triangle_opposites,
                triangle_frictions: &triangle_frictions,
            },
        );
        let pipeline_part = squishy_volumes_gpu::Step::new(&gpu_context, settings);

        GpuState {
            gpu_context,
            pipeline_part,
            next_input,
        }
    }
}

pub struct GpuState {
    pub gpu_context: squishy_volumes_gpu::GpuContext,
    pub pipeline_part: squishy_volumes_gpu::Step,
    pub next_input: squishy_volumes_gpu::step::Input,
}
