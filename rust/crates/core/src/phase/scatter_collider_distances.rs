// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    mem::take,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::channel,
    },
    thread::spawn,
};

use anyhow::{Context as _, Result, bail};
use nalgebra::Vector3;
use rayon::iter::{
    IndexedParallelIterator as _, IntoParallelIterator, IntoParallelRefIterator as _,
    IntoParallelRefMutIterator, ParallelExtend, ParallelIterator,
};
use rustc_hash::FxHashMap;
use tracing::info;

use crate::{
    math::RASTERIZATION_LAYERS,
    profile,
    rasterization::{RasterizationVertex, rasterize},
    state::{
        ObjectIndex,
        grids::{ColliderInfos, GridNodeCollider, Mutex, Rasterized},
    },
};

use super::{PhaseInput, State};

impl State {
    // Splat collider distances into the grid.
    // Each grid node then contains signed distance and normal information for each collider.
    // But only if it's close enough.
    pub(super) fn scatter_collider_distances(
        mut self,
        phase_input: &mut PhaseInput,
    ) -> Result<Self> {
        profile!("scatter_collider_distances");
        let grid_node_size = phase_input.consts.scaled_grid_node_size();

        let collider_input = &self
            .interpolated_input
            .as_ref()
            .expect("interpolated input missing")
            .collider_input;

        {
            profile!("reset");
            self.grid_collider
                .par_iter_mut()
                .for_each(|(_, grid_node)| {
                    let GridNodeCollider::Ref(mut infos) = take(grid_node) else {
                        panic!("Collider node wasn't ref");
                    };
                    infos.clear();
                    *grid_node = GridNodeCollider::Mut(Mutex(infos.into()));
                });
        }

        let collector = {
            profile!("scatter");
            let (tx, rx) = channel::<(Vector3<i32>, (u8, Rasterized))>();
            let collector = spawn(move || -> FxHashMap<Vector3<i32>, ColliderInfos> {
                let mut new_entries: FxHashMap<Vector3<i32>, ColliderInfos> = Default::default();
                while let Ok((grid_idx, (collider_idx, rasterized))) = rx.recv() {
                    let grid_node = new_entries.entry(grid_idx).or_default();
                    if let Some(info) = grid_node.get_mut(&collider_idx) {
                        if info.distance_abs() < rasterized.distance_abs() {
                            continue;
                        }
                        *info = rasterized;
                    } else {
                        grid_node.insert(collider_idx, rasterized);
                    }
                }

                new_entries.values_mut().for_each(|grid_node| {
                    grid_node.retain(|_, rasterized| matches!(rasterized, Rasterized::Valid(_)))
                });
                new_entries
            });

            for (name, input) in collider_input.iter() {
                let object_index = self.name_map.get(name).context("Missing object")?;
                let ObjectIndex::Collider(collider_index) = object_index.clone() else {
                    bail!("Wrong object type");
                };
                let collider_index = collider_index as u8;

                let make_rasterization_vertex = |index: usize| RasterizationVertex {
                    position: &input.vertex_positions[index],
                    velocity: &input.vertex_velocities[index],
                    normal: &input.vertex_normals[index],
                };

                input
                    .triangles
                    .par_iter()
                    .zip(&input.triangle_frictions)
                    .for_each(|(&[a, b, c], friction)| {
                        let order_edge = |[a, b]: [u32; 2]| if a < b { [a, b] } else { [b, a] };
                        let pick_other = |a: u32| {
                            move |&[b, c]: &[u32; 2]| {
                                &input.vertex_positions[if b != a { b } else { c } as usize]
                            }
                        };
                        let opposite_d = input
                            .edges_with_opposites
                            .get(&order_edge([a, b]))
                            .map(pick_other(c));
                        let opposite_e = input
                            .edges_with_opposites
                            .get(&order_edge([b, c]))
                            .map(pick_other(a));
                        let opposite_f = input
                            .edges_with_opposites
                            .get(&order_edge([c, a]))
                            .map(pick_other(b));

                        for (grid_idx, rasterized) in rasterize(
                            grid_node_size,
                            RASTERIZATION_LAYERS,
                            [
                                make_rasterization_vertex(a as usize),
                                make_rasterization_vertex(b as usize),
                                make_rasterization_vertex(c as usize),
                            ],
                            [opposite_d, opposite_e, opposite_f],
                            *friction,
                        ) {
                            let Some(grid_node) = self.grid_collider.get(&grid_idx) else {
                                tx.send((grid_idx, (collider_index, rasterized)))
                                    .expect("collider collector died");
                                continue;
                            };

                            let mut grid_node = grid_node.assume_mut().lock();
                            if let Some(info) = grid_node.get_mut(&collider_index) {
                                if info.distance_abs() < rasterized.distance_abs() {
                                    continue;
                                }
                                *info = rasterized;
                            } else {
                                grid_node.insert(collider_index, rasterized);
                            }
                        }
                    });
            }
            collector
        };

        {
            profile!("transition");
            self.grid_collider
                .par_iter_mut()
                .for_each(|(_, grid_node)| {
                    let GridNodeCollider::Mut(Mutex(mutex)) = take(grid_node) else {
                        panic!("Collider node was't mut");
                    };
                    let mut infos = mutex.into_inner().unwrap();
                    infos.retain(|_, rasterized| matches!(rasterized, Rasterized::Valid(_)));
                    *grid_node = GridNodeCollider::Ref(infos);
                });
        }

        {
            profile!("prune");
            self.grid_collider
                .retain(|_, infos| !infos.assume_ref().is_empty());
        }

        let new_entries = {
            profile!("join");
            collector.join().unwrap()
        };

        {
            profile!("extend");
            self.grid_collider.par_extend(
                new_entries
                    .into_par_iter()
                    .map(|(grid_idx, infos)| (grid_idx, GridNodeCollider::Ref(infos))),
            );
        }

        Ok(self)
    }
}
