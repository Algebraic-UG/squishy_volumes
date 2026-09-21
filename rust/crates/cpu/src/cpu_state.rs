// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZero;

use squishy_volumes_file_frame::{IoState, ParticleFlags};
use squishy_volumes_util::AnimatedGlobals;
use squishy_volumes_xpu::FrameInput;

use super::*;

pub struct CpuState {
    pub(crate) time: f64,

    pub(crate) animated_globals: AnimatedGlobals,

    pub(crate) adaptive_time_step_state: AdaptiveTimeStepState,
    pub(crate) phase: Phase,
    pub(crate) particles: Particles,

    pub(crate) frame_input: FrameInput,

    pub(crate) grid_nodes: GridNodes,

    pub(crate) interpolated_input: Option<InterpolatedInput>,
}

impl CpuState {
    pub fn new(
        frame: usize,
        io_state: IoState,
        input_reader: squishy_volumes_file_input::InputReader,
    ) -> Result<Self, Error> {
        let time = io_state.time;
        let animated_globals = io_state.animated_globals;

        let sort_map: Vec<u32> = (0..io_state.particles.flags.len() as u32).collect();
        let reverse_sort_map = sort_map.clone();
        let flags = io_state.particles.flags;
        let parameters = io_state.particles.parameters;
        let initial_positions = bytemuck::try_cast_vec(io_state.particles.initial_positions)
            .map_err(|_| Error::CastFailed)?;
        let positions =
            bytemuck::try_cast_vec(io_state.particles.positions).map_err(|_| Error::CastFailed)?;
        let position_gradients = bytemuck::try_cast_vec(io_state.particles.position_gradients)
            .map_err(|_| Error::CastFailed)?;
        let velocities =
            bytemuck::try_cast_vec(io_state.particles.velocities).map_err(|_| Error::CastFailed)?;
        let velocity_gradients = bytemuck::try_cast_vec(io_state.particles.velocity_gradients)
            .map_err(|_| Error::CastFailed)?;
        let elastic_energies = io_state.particles.elastic_energies;
        let collider_bits = io_state.particles.collider_bits;

        let particles = Particles {
            sort_map,
            reverse_sort_map,
            flags,
            parameters,
            initial_positions,
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
            elastic_energies,
            collider_bits,
        };

        let frame_input = FrameInput::new(
            input_reader,
            io_state.particles.goal_positions,
            io_state.collider,
            frame,
        )?;

        Ok(Self {
            time,
            animated_globals,

            particles,

            frame_input,

            phase: Default::default(),
            adaptive_time_step_state: Default::default(),
            grid_nodes: Default::default(),
            interpolated_input: Default::default(),
        })
    }

    pub fn to_io_state(&self, store_bvh: bool, store_grid: bool) -> Result<IoState, Error> {
        let time = self.time;
        let animated_globals = self.animated_globals;

        fn permute<T: Copy, U: std::convert::From<T>>(
            permutation: &[u32],
            to_permute: &[T],
        ) -> Vec<U> {
            permutation
                .iter()
                .map(|index| to_permute[*index as usize].into())
                .collect()
        }

        let p = &self.particles.reverse_sort_map;
        let flags: Vec<ParticleFlags> = permute(p, &self.particles.flags);
        let parameters = permute(p, &self.particles.parameters);
        let elastic_energies = permute(p, &self.particles.elastic_energies);
        let collider_bits = permute(p, &self.particles.collider_bits);
        let positions = permute(p, &self.particles.positions);
        let position_gradients = permute(p, &self.particles.position_gradients);
        let velocities = permute(p, &self.particles.velocities);
        let velocity_gradients = permute(p, &self.particles.velocity_gradients);
        let initial_positions = permute(p, &self.particles.initial_positions);

        let goal_positions = bytemuck::cast_slice(&self.frame_input.goal_positions_end()).to_vec();

        let particles = squishy_volumes_file_frame::Particles {
            flags,
            parameters,
            elastic_energies,
            collider_bits,
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
            initial_positions,
            goal_positions,
        };

        let collider = self.frame_input.collider_start().to_io_collider();

        let grid_nodes = store_grid.then(|| {
            let (node_ids, collider_bits) = self
                .grid_nodes
                .keys
                .iter()
                .map(
                    |GridKey {
                         node_id,
                         collider_bits,
                     }|
                     -> ([i32; 3], u32) { (*node_id.as_ref(), *collider_bits) },
                )
                .unzip();
            let masses = self.grid_nodes.masses.clone();
            let velocites = bytemuck::cast_slice(&self.grid_nodes.velocities).to_vec();

            squishy_volumes_file_frame::GridNodes {
                node_ids,
                collider_bits,
                masses,
                velocites,
            }
        });

        let bvh = store_bvh.then(|| self.frame_input.bvh().clone());

        Ok(IoState {
            time,
            animated_globals,
            particles,
            collider,
            bvh,
            grid_nodes,
        })
    }
}

pub struct CpuRunParameters {
    pub target_time: f64,
    pub max_time_step: f32,
    pub adaptive_time_steps: bool,
    pub store_bvh: bool,
    pub store_grid: bool,
}

impl CpuState {
    pub fn produce_next_state(
        &mut self,
        harness: &squishy_volumes_xpu::Harness,
        frame_input: &squishy_volumes_xpu::FrameInput,
        CpuRunParameters {
            target_time,
            max_time_step,
            adaptive_time_steps,
            store_bvh,
            store_grid,
        }: CpuRunParameters,
    ) -> Result<(squishy_volumes_file_frame::IoState, Result<(), Error>), Error> {
        squishy_volumes_util::profile!("produce_next_state");

        let harness = harness.scope(
            "Simulation Milliseconds to Next Frame".to_string(),
            NonZero::new(((1000. * frame_input.consts().seconds_per_frame()) as usize).max(1))
                .unwrap(),
        )?;

        self.adaptive_time_step_state.max_time_step = max_time_step;

        while self.time < target_time {
            harness.check()?;

            if self.adaptive_time_step_state.allowed_time_step() == 0. {
                return Err(Error::ZeroTimeStep);
            }

            let run_phase = adaptive_time_steps
                || (self.phase != Phase::LimitTimeStepBeforeForce
                    && self.phase != Phase::LimitTimeStepBeforeIntegrate);
            if run_phase {
                match self.run_phase(frame_input) {
                    error @ Err(Error::EnergyError(energy_error)) => {
                        tracing::warn!(?energy_error, "Encountered an energy error.");
                        return Ok((self.to_io_state(store_bvh, store_grid)?, error));
                    }
                    x => x?,
                }
            }

            self.phase = self.phase.cycle();
            if self.phase == Default::default() {
                self.time += self.adaptive_time_step_state.allowed_time_step() as f64;
            }
            harness.step_to(
                ((self.time % frame_input.consts().seconds_per_frame()) * 1000.) as usize,
            )?;
        }

        Ok((self.to_io_state(store_bvh, store_grid)?, Ok(())))
    }
}
