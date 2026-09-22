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
mod cull_particles;
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
    pub fn run_phase(&mut self) -> Result<(), Error> {
        match self.phase {
            Phase::InterpolateInput => self.interpolate_input()?,
            Phase::Sort => self.sort(),
            Phase::Collide => self.collide(),
            Phase::ExternalForce => self.external_force()?,
            Phase::UpdateGridNodes => self.update_grid_nodes(),
            Phase::LimitTimeStepBeforeForce => self.limit_time_step_before_force(),
            Phase::ScatterMomentum => self.scatter_momentum(),
            Phase::MeldGrid => self.meld_grid(),
            Phase::CollectVelocity => self.collect_velocity(),
            Phase::LimitTimeStepBeforeIntegrate => self.limit_time_step_before_integrate(),
            Phase::AdvanceParticles => self.advance_particles()?,
            Phase::CullParticles => self.cull_particles(),
        }

        Ok(())
    }
}
