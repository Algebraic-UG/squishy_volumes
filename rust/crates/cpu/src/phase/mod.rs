// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
