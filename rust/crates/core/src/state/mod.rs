// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, iter::once};

use crate::{
    input_interpolation::InterpolatedInput, phase::Phase, state::grids::GridCollider,
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

mod errors;

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
}
