// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::VecDeque;

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;
use strum::{EnumIter, IntoEnumIterator};

use crate::input_interpolation::InputInterpolation;
use crate::{input_file::InputConsts, state::State};

use crate::profile;

mod advect_particles;
mod collect_insides;
mod collect_velocity;
mod cull_particles;
mod external_force;
mod goal_forces;
mod implicit_solve;
mod interpolate_input;
mod limit_time_step;
mod meld_grid;
mod scatter_momentum;
mod sort;
mod update_momentum_maps;

// XXX: Order matters!
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, EnumIter, Serialize, Deserialize)]
pub enum Phase {
    #[default]
    InterpolateInput,
    Sort,
    CollectInsides,
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
    pub fn function(self) -> fn(State, &mut PhaseInput) -> Result<State> {
        match self {
            Self::InterpolateInput => State::interpolate_input,
            Self::Sort => State::sort,
            Self::CollectInsides => State::collect_insides,
            Self::ExternalForce => State::external_force,
            Self::GoalForces => State::goal_forces,
            Self::UpdateMomentumMaps => State::update_momentum_maps,
            Self::LimitTimeStepBeforeForce => State::limit_time_step_before_force,
            Self::ScatterMomentum => State::scatter_momentum::<false>,
            Self::ScatterMomentumExplicit => State::scatter_momentum::<true>,
            Self::MeldGrid => State::meld_grid,
            Self::ImplicitSolve => State::implicit_solve,
            Self::CollectVelocity => State::collect_velocity,
            Self::LimitTimeStepBeforeIntegrate => State::limit_time_step_before_integrate,
            Self::AdvectParticles => State::advect_particles,
            Self::CullParticles => State::cull_particles,
        }
    }

    pub fn cycle(self) -> Self {
        let mut it = Self::iter().cycle();
        while it.next() != Some(self) {}
        it.next().unwrap()
    }
}

pub struct PhaseInput {
    pub consts: InputConsts,

    pub input_interpolation: InputInterpolation,

    pub max_time_step: T,
    pub time_step_by_velocity: Option<T>,
    pub time_step_by_deformation: Option<T>,
    pub time_step_by_isolated: Option<T>,
    pub time_step_by_sound: Option<T>,
    pub time_step: T,
    pub time_step_prior: VecDeque<T>,
    pub adaptive_time_steps: bool,
    pub explicit: bool,
}

impl State {
    pub fn next(mut self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("next");

        ensure!(phase_input.time_step != 0.);

        let phase = self.phase;

        self = {
            let run_phase = if phase_input.explicit {
                !matches!(phase, Phase::ScatterMomentum | Phase::ImplicitSolve)
            } else {
                !matches!(phase, Phase::ScatterMomentumExplicit)
            } && (phase_input.adaptive_time_steps || {
                !matches!(
                    phase,
                    Phase::LimitTimeStepBeforeForce | Phase::LimitTimeStepBeforeIntegrate
                )
            });

            if run_phase {
                self.phase.function()(self, phase_input)
                    .with_context(|| format!("Failed in phase: {phase:?}"))?
            } else {
                self
            }
        };
        self.phase = self.phase.cycle();

        if self.phase == Default::default() {
            self.time += phase_input.time_step as f64;
        }

        Ok(self)
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
}
