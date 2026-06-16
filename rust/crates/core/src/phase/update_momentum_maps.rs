// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use nalgebra::Vector3;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rustc_hash::FxHashMap;
use std::{sync::mpsc::channel, thread::spawn};

use crate::{
    kernels::{kernel_quadratic_unrolled, position_to_shift_quadratic},
    profile,
    state::{
        grids::{GridKey, Mutex},
        particles::ParticleState,
    },
};

use super::{PhaseInput, State};

impl State {
    // Update the hash map that allows to index into all the vectors of each momentum grid
    // with the node's 3d integer position. The data vectors are effectively invalidated.
    pub fn update_momentum_maps(mut self, phase_input: &mut PhaseInput) -> Result<Self> {
        profile!("update_momentum_maps");
        let grid_node_size = phase_input.consts.scaled_grid_node_size();

        {
            // remove those entries that didn't receive any particles
            profile!("prune");
            self.grid
                .map
                .retain(|_, idx| !self.grid.contributors[*idx].get_mut().unwrap().is_empty());
        }

        {
            // the pruning breaks the indexing
            profile!("re-index");
            self.grid
                .map
                .values_mut()
                .enumerate()
                .for_each(|(i, e)| *e = i);
        }

        // to avoid frequent reallocations we add nodes with generous capacity
        let initial_capacity = 1 << 4;

        {
            // this effectively clears all the contributors
            profile!("prepare");
            self.grid.prepare_contributors(initial_capacity);
        }

        // we start a collector thread for the new entries
        // this is a bit tricky: we need to make sure that the new entries' indices are offset
        // and when we access the new contributors, we need to subtract that offset
        let grid_index_offset = self.grid.map.len();
        let mut next_grid_index = grid_index_offset;
        let (tx, rx) = channel();
        let collector = spawn(move || {
            let mut map: FxHashMap<GridKey, usize> = Default::default();
            let mut contributors: Vec<Mutex<Vec<usize>>> = Default::default();
            while let Ok((grid_key, particle_index)) = rx.recv() {
                let grid_index = *map.entry(grid_key).or_insert_with(|| {
                    contributors.push(Vec::with_capacity(initial_capacity).into());
                    let grid_index = next_grid_index;
                    next_grid_index += 1;
                    grid_index
                });
                contributors[grid_index - grid_index_offset]
                    .get_mut()
                    .unwrap()
                    .push(particle_index);
            }
            (map, contributors)
        });

        // generate grid from particles
        self.particles
            .positions
            .par_iter()
            .zip(&self.particles.collider_bits)
            .enumerate()
            .zip(&self.particles.states)
            .filter_map(|(e, state)| (*state != ParticleState::Tombstoned).then_some(e))
            .for_each(|(particle_index, (position, &collider_bits))| {
                let shift = position_to_shift_quadratic(position, grid_node_size);
                kernel_quadratic_unrolled!(|grid_id| {
                    let node_id = grid_id + shift;
                    let key = GridKey {
                        node_id,
                        collider_bits,
                    };

                    if let Some(grid_index) = self.grid.map.get(&key) {
                        // if the entry exists, we register
                        self.grid.contributors[*grid_index]
                            .lock()
                            .push(particle_index);
                    } else {
                        // otherwise it's handled in the collector
                        tx.send((key, particle_index)).expect("collector died");
                    }
                });
            });

        {
            // add the new entries
            profile!("collect");
            drop(tx);
            let (map, mut contributors) = collector.join().unwrap();
            self.grid.map.extend(map.into_iter());
            self.grid.contributors.append(&mut contributors);
        }

        Ok(self)
    }
}
