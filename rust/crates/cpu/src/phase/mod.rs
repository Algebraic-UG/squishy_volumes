// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use strum::{EnumIter, IntoEnumIterator as _};

use super::*;

mod collide;
mod external_force;
mod interpolate_input;
mod sort;

// XXX: Order matters!
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, EnumIter, PartialOrd)]
pub enum Phase {
    #[default]
    InterpolateInput,
    Sort,
    Collide,
    ExternalForce,
    GoalForces,
    UpdateMomentumMaps,
    LimitTimeStepBeforeForce,
    ScatterMomentum,
    ScatterMomentumExplicit,
    MeldGrid,
    ImplicitSolve,
    CollectVelocity,
    LimitTimeStepBeforeIntegrate,
    AdvectParticles,
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
        time_step: f32,
        frame_input: &squishy_volumes_xpu::FrameInput,
    ) -> Result<(), Error> {
        match self.phase {
            Phase::InterpolateInput => self.interpolate_input(frame_input),
            Phase::Sort => self.sort(frame_input.consts().scaled_grid_node_size()),
            Phase::Collide => self.collide(time_step, frame_input),
            Phase::ExternalForce => todo!(),
            Phase::GoalForces => todo!(),
            Phase::UpdateMomentumMaps => todo!(),
            Phase::LimitTimeStepBeforeForce => todo!(),
            Phase::ScatterMomentum => todo!(),
            Phase::ScatterMomentumExplicit => todo!(),
            Phase::MeldGrid => todo!(),
            Phase::ImplicitSolve => todo!(),
            Phase::CollectVelocity => todo!(),
            Phase::LimitTimeStepBeforeIntegrate => todo!(),
            Phase::AdvectParticles => todo!(),
            Phase::CullParticles => todo!(),
        }
    }
}
