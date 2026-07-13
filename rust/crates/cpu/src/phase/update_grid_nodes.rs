// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use rayon::{
    iter::{
        IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator as _,
        ParallelIterator,
    },
    slice::ParallelSliceMut as _,
};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use squishy_volumes_file_frame::ParticleFlags;
use squishy_volumes_util::profile;
use std::{
    sync::{Mutex, mpsc::channel},
    thread::spawn,
};

use super::*;

impl CpuState {
    // Update the hash map that allows to index into all the vectors of each momentum grid
    // with the node's 3d integer position. The data vectors are effectively invalidated.
    pub fn update_grid_nodes(&mut self, grid_node_size: f32) {
        profile!("update_grid_nodes");

        {
            // remove those entries that didn't receive any particles
            profile!("prune");
            self.grid_nodes.map.retain(|_, idx| {
                !self.grid_nodes.contributors[*idx as usize]
                    .get_mut()
                    .unwrap()
                    .is_empty()
            });
        }

        {
            // the pruning breaks the indexing
            profile!("re-index");
            self.grid_nodes
                .map
                .values_mut()
                .enumerate()
                .for_each(|(i, e)| *e = i as u32);
        }

        {
            profile!("prepare");
            self.grid_nodes
                .contributors
                .drain(self.grid_nodes.map.len()..);
            self.grid_nodes
                .contributors
                .par_iter_mut()
                .for_each(|v| v.get_mut().unwrap().clear());
            for _ in self.grid_nodes.contributors.len()..self.grid_nodes.map.len() {
                self.grid_nodes.contributors.push(Default::default());
            }
        }

        // we start a collector thread for the new entries
        // this is a bit tricky: we need to make sure that the new entries' indices are offset
        // and when we access the new contributors, we need to subtract that offset
        let grid_index_offset = self.grid_nodes.map.len() as u32;
        let mut next_grid_index = grid_index_offset;
        let (tx, rx) = channel();
        let collector = spawn(move || {
            let mut map: FxHashMap<GridKey, u32> = Default::default();
            let mut contributors: Vec<Mutex<SmallVec<[u32; 16]>>> = Default::default();
            while let Ok((grid_key, particle_index)) = rx.recv() {
                let grid_index = *map.entry(grid_key).or_insert_with(|| {
                    contributors.push(SmallVec::new().into());
                    let grid_index = next_grid_index;
                    next_grid_index += 1;
                    grid_index
                });
                contributors[(grid_index - grid_index_offset) as usize]
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
            .zip(&self.particles.flags)
            .filter_map(|(e, flags)| (!flags.contains(ParticleFlags::TOMBSTONED)).then_some(e))
            .for_each(|(particle_index, (position, &collider_bits))| {
                let shift = position_to_shift_quadratic(position, grid_node_size);
                kernel_quadratic_unrolled!(|grid_id| {
                    let node_id = grid_id + shift;
                    let key = GridKey {
                        node_id,
                        collider_bits,
                    };

                    if let Some(grid_index) = self.grid_nodes.map.get(&key) {
                        // if the entry exists, we register
                        self.grid_nodes.contributors[*grid_index as usize]
                            .lock()
                            .unwrap()
                            .push(particle_index as u32);
                    } else {
                        // otherwise it's handled in the collector
                        tx.send((key, particle_index as u32))
                            .expect("collector died");
                    }
                });
            });

        {
            // add the new entries
            profile!("collect");
            drop(tx);
            let (map, mut contributors) = collector.join().unwrap();
            self.grid_nodes.map.extend(map);
            self.grid_nodes.contributors.append(&mut contributors);
        }

        {
            profile!("sort keys");
            let mut keys: Vec<(GridKey, u32)> = self
                .grid_nodes
                .map
                .iter()
                .map(|(key, &index)| (key.clone(), index))
                .collect();
            keys.par_sort_by_key(|keys| keys.1);
            self.grid_nodes.keys = keys.into_iter().map(|key| key.0).collect();
        }

        {
            profile!("multi map");
            self.grid_nodes.multi_map.clear();
            for (index, GridKey { node_id, .. }) in self.grid_nodes.keys.iter().enumerate() {
                self.grid_nodes
                    .multi_map
                    .entry(*node_id)
                    .or_default()
                    .push(index as u32);
            }
        }
    }
}
