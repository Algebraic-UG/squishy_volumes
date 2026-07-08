// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use strum::{EnumIter, IntoEnumIterator as _};

use super::*;

mod advance_particles;
mod collect_velocity;
mod collide;
mod external_force;
mod interpolate_input;
mod limit_time_step;
mod meld_grid;
mod scatter_momentum;
mod sort;
mod update_grid_nodes;

// XXX: Order matters!
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, EnumIter, PartialOrd)]
pub enum Phase {
    #[default]
    InterpolateInput,
    Sort,
    Collide,
    ExternalForce,
    UpdateGridNodes,
    LimitTimeStepBeforeForce,
    ScatterMomentum,
    MeldGrid,
    CollectVelocity,
    LimitTimeStepBeforeIntegrate,
    AdvanceParticles,
    CullParticles,
}

impl Phase {
    pub fn cycle(self) -> Self {
        let mut it = Self::iter().cycle();
        while it.next() != Some(self) {}
        it.next().unwrap()
    }
}

impl CpuState {
    pub fn run_phase(
        &mut self,
        frame_input: &squishy_volumes_xpu::FrameInput,
    ) -> Result<(), Error> {
        let grid_node_size = frame_input.consts().scaled_grid_node_size();
        match self.phase {
            Phase::InterpolateInput => self.interpolate_input(frame_input)?,
            Phase::Sort => self.sort(grid_node_size),
            Phase::Collide => self.collide(frame_input),
            Phase::ExternalForce => self.external_force(frame_input)?,
            Phase::UpdateGridNodes => self.update_grid_nodes(grid_node_size),
            Phase::LimitTimeStepBeforeForce => self.limit_time_step_before_force(grid_node_size),
            Phase::ScatterMomentum => self.scatter_momentum(grid_node_size),
            Phase::MeldGrid => self.meld_grid(),
            Phase::CollectVelocity => self.collect_velocity(grid_node_size),
            Phase::LimitTimeStepBeforeIntegrate => {
                self.limit_time_step_before_integrate(grid_node_size)
            }
            Phase::AdvanceParticles => self.advance_particles()?,
            Phase::CullParticles => todo!(),
        }

        Ok(())
    }
}
