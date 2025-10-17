// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Context, Result, bail, ensure};
use nalgebra::Vector3;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;
use std::{
    collections::{BTreeMap, VecDeque},
    iter::once,
    num::NonZero,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use strum::{EnumIter, IntoEnumIterator};
use tracing::info;

use crate::{
    api::{ObjectSettings, ObjectWithData, Setup, StateStats},
    report::{Report, ReportInfo},
    simulation::{
        collider::ColliderConstruction, fluid::FluidConstruction, solid::SolidConstruction,
    },
};

use super::{
    collider::Collider,
    fluid::Fluid,
    grids::{GridColliderDistances, GridMomentum, GridNodeColliderDistances},
    particles::Particles,
    solid::Solid,
};

#[cfg(feature = "profile")]
pub use coarse_prof::profile;
#[cfg(not(feature = "profile"))]
macro_rules! profile {
    ($name:expr) => {};
}
#[cfg(not(feature = "profile"))]
pub(crate) use profile;

mod advect_particles;
pub(super) mod attributes;
mod collect_insides;
mod collect_velocity;
mod conform_to_colliders;
mod external_force;
mod implicit_solve;
mod limit_time_step;
mod move_collider;
mod register_contributors;
mod scatter_collider_distances;
mod scatter_momentum;
mod sort;
mod update_momentum_maps;

#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    time: f64,
    phase: Phase,
    name_map: BTreeMap<String, ObjectIndex>,

    particles: Particles,

    solid_objects: Vec<Solid>,
    fluid_objects: Vec<Fluid>,
    collider_objects: Vec<Collider>,

    grid_collider_distances: GridColliderDistances,

    grid_momentum: GridMomentum,
    grid_collider_momentums: Vec<GridMomentum>,
}

#[derive(Clone)]
pub struct PhaseInput {
    pub max_time_step: T,
    pub time_step_by_velocity: Option<T>,
    pub time_step_by_deformation: Option<T>,
    pub time_step_by_isolated: Option<T>,
    pub time_step_by_sound: Option<T>,
    pub time_step_by_sound_simple: Option<T>,
    pub time_step: T,
    pub time_step_prior: VecDeque<T>,
    pub adaptive_time_steps: bool,
    pub explicit: bool,
    pub debug_mode: bool,
    pub setup: Arc<Setup>,
}

// XXX: Order matters!
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, EnumIter, Serialize, Deserialize)]
pub enum Phase {
    #[default]
    Sort,
    ScatterColliderDistances,
    CollectInsides,
    UpdateMomentumMaps,
    RegisterContributors,
    LimitTimeStepBeforeForce,
    ScatterMomentum,
    ScatterMomentumExplicit,
    ExternalForce,
    ConformToColliders,
    ImplicitSolve,
    CollectVelocity,
    LimitTimeStepBeforeIntegrate,
    AdvectParticles,
    MoveCollider,
}

impl Phase {
    pub fn function(self) -> fn(State, &mut PhaseInput) -> Result<State> {
        match self {
            Self::Sort => State::sort,
            Self::ScatterColliderDistances => State::scatter_collider_distances,
            Self::CollectInsides => State::collect_insides,
            Self::UpdateMomentumMaps => State::update_momentum_maps,
            Self::RegisterContributors => State::register_contributors,
            Self::LimitTimeStepBeforeForce => State::limit_time_step_before_force,
            Self::ScatterMomentum => State::scatter_momentum::<false>,
            Self::ScatterMomentumExplicit => State::scatter_momentum::<true>,
            Self::ExternalForce => State::external_force,
            Self::ConformToColliders => State::conform_to_colliders,
            Self::ImplicitSolve => State::implicit_solve,
            Self::CollectVelocity => State::collect_velocity,
            Self::LimitTimeStepBeforeIntegrate => State::limit_time_step_before_integrate,
            Self::AdvectParticles => State::advect_particles,
            Self::MoveCollider => State::move_collider,
        }
    }

    pub fn cycle(self) -> Self {
        let mut it = Self::iter().cycle();
        while it.next() != Some(self) {}
        it.next().unwrap()
    }
}

#[derive(Clone, PartialEq, PartialOrd, Ord, Eq, Serialize, Deserialize)]
enum ObjectIndex {
    Solid(usize),
    Fluid(usize),
    Collider(usize),
}

impl State {
    pub fn new(
        run: Arc<AtomicBool>,
        report: Report,
        Setup { settings, objects }: &Setup,
    ) -> Result<Self> {
        let report = report.new_sub(ReportInfo {
            name: "Initializing Objects".to_string(),
            completed_steps: 0,
            steps_to_completion: NonZero::new(objects.len().max(1)).unwrap(),
        });

        let mut name_map = BTreeMap::new();
        let mut particles = Particles::default();
        let mut solid_objects = Vec::new();
        let mut fluid_objects = Vec::new();
        let mut collider_objects = Vec::new();
        for ObjectWithData {
            object,
            mesh,
            scripted_frames,
        } in objects
        {
            ensure!(run.load(Ordering::Relaxed), "Cancelled");

            let name = object.name.clone();
            info!(name, "object");

            if object.scale.iter().any(|c| *c < 0.) {
                bail!("negative scaling isn't supported, please check '{name}'");
            }

            let kinematic = object
                .clone()
                .try_into()
                .context("Kinematic construction")?;
            let object_idx = match &object.settings {
                ObjectSettings::Solid(object_settings) => {
                    let solid = Solid::new(SolidConstruction {
                        name: &name,
                        run: run.clone(),
                        report: report.clone(),
                        settings,
                        kinematic,
                        object_settings: object_settings.clone(),
                        mesh,
                        particles: &mut particles,
                    })
                    .with_context(|| format!("Solid creation: '{name}'"))?;
                    let object_idx = ObjectIndex::Solid(solid_objects.len());
                    solid_objects.push(solid);
                    object_idx
                }
                ObjectSettings::Fluid(object_settings) => {
                    let fluid = Fluid::new(FluidConstruction {
                        name: &name,
                        run: run.clone(),
                        report: report.clone(),
                        settings,
                        kinematic,
                        object_settings: object_settings.clone(),
                        mesh,
                        particles: &mut particles,
                    })
                    .with_context(|| format!("Fluid creation: '{name}'"))?;
                    let object_idx = ObjectIndex::Fluid(fluid_objects.len());
                    fluid_objects.push(fluid);
                    object_idx
                }
                ObjectSettings::Collider(object_settings) => {
                    let collider = Collider::new(ColliderConstruction {
                        name: &name,
                        run: run.clone(),
                        report: report.clone(),
                        settings,
                        kinematic,
                        object_settings: object_settings.clone(),
                        mesh,
                        scripted_frames: scripted_frames.clone(),
                    })
                    .with_context(|| format!("Collider creation: '{name}'"))?;
                    let object_idx = ObjectIndex::Collider(collider_objects.len());
                    collider_objects.push(collider);
                    object_idx
                }
            };
            ensure!(name_map.insert(name, object_idx).is_none());
            report.step();
        }
        let grid_collider_momentums = vec![Default::default(); collider_objects.len()];

        info!(
            solid_objects = solid_objects.len(),
            solid_particles = solid_objects
                .iter()
                .map(|solid| solid.particles.len())
                .sum::<usize>(),
            fluid_objects = fluid_objects.len(),
            fluid_particles = 0, // TODO
            collider_objects = collider_objects.len(),
            collider_particles = collider_objects
                .iter()
                .map(|collider| collider.surface_samples.len())
                .sum::<usize>(),
        );

        Ok(Self {
            time: 0.,
            phase: Default::default(),
            name_map,
            particles,
            solid_objects,
            fluid_objects,
            collider_objects,
            grid_collider_distances: Default::default(),
            grid_momentum: Default::default(),
            grid_collider_momentums,
        })
    }

    pub fn next(mut self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("next");

        ensure!(phase_input.time_step != 0.);

        let phase = self.phase;

        self = {
            let run_phase = if phase_input.explicit {
                !matches!(phase, Phase::ScatterMomentum | Phase::ImplicitSolve)
            } else {
                !matches!(phase, Phase::ScatterMomentumExplicit)
            };

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

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    fn grid_momentums(&self) -> impl Iterator<Item = &GridMomentum> {
        once(&self.grid_momentum).chain(self.grid_collider_momentums.iter())
    }

    fn grid_momentums_mut(&mut self) -> impl Iterator<Item = &mut GridMomentum> {
        once(&mut self.grid_momentum).chain(self.grid_collider_momentums.iter_mut())
    }

    pub fn stats(&self) -> StateStats {
        let total_particle_count = self.particles.reverse_sort_map.len()
            + self
                .collider_objects
                .iter()
                .map(|collider| collider.surface_samples.len())
                .sum::<usize>();
        let total_grid_node_count = self.grid_momentums().map(|grid| grid.masses.len()).sum();
        let per_object_count = self
            .name_map
            .iter()
            .map(|(name, object_idx)| {
                (
                    name.clone(),
                    match object_idx {
                        ObjectIndex::Solid(solid_idx) => {
                            self.solid_objects[*solid_idx].particles.len()
                        }
                        ObjectIndex::Fluid(fluid_idx) => {
                            self.fluid_objects[*fluid_idx].particles.len()
                        }
                        ObjectIndex::Collider(collider_idx) => {
                            self.collider_objects[*collider_idx].surface_samples.len()
                        }
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
}

fn check_shifted_quadratic(shifted: Vector3<T>) -> bool {
    shifted.x >= 0.5
        && shifted.x <= 1.5
        && shifted.y >= 0.5
        && shifted.y <= 1.5
        && shifted.z >= 0.5
        && shifted.z <= 1.5
}

#[allow(unused)]
fn check_shifted_cubic(shifted: Vector3<T>) -> bool {
    shifted.x >= 1.
        && shifted.x <= 2.
        && shifted.y >= 1.
        && shifted.y <= 2.
        && shifted.z >= 1.
        && shifted.z <= 2.
}

fn find_worst_incompatibility(
    collider_insides: &FxHashMap<usize, bool>,
    grid_node: &GridNodeColliderDistances,
) -> Option<usize> {
    collider_insides
        .iter()
        .filter_map(|(collider_idx, inside)| {
            Some((
                *collider_idx,
                grid_node
                    .weighted_distances
                    .get(collider_idx)
                    .and_then(|weighted_distance| {
                        (inside ^ (weighted_distance.distance < 0.))
                            .then_some(weighted_distance.distance.abs())
                    })?,
            ))
        })
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(collider_idx, _)| collider_idx)
}
