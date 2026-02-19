// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::{BTreeMap, hash_map::Entry};

use anyhow::{Result, bail, ensure};
use nalgebra::{Unit, Vector3};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;

use crate::{
    input_file::{InputFrame, InputReader},
    math::NORMALIZATION_EPS,
    profile,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct InterpolatedInput {
    pub gravity: Vector3<T>,
    pub particles_input: BTreeMap<String, InterpolatedInputParticles>,
    pub collider_input: BTreeMap<String, InterpolatedInputCollider>,
}

impl InterpolatedInput {
    fn new(
        InputFrame {
            gravity,
            particles_inputs,
            collider_inputs,
        }: InputFrame,
    ) -> Result<Self> {
        let particles_input = particles_inputs
            .iter()
            .map(|(name, input)| {
                let goal_positions = input
                    .goal_positions
                    .chunks_exact(3)
                    .map(Vector3::from_column_slice)
                    .collect();

                let goal_stiffnesses = input.goal_stiffnesses.clone();
                (
                    name.clone(),
                    InterpolatedInputParticles {
                        goal_positions,
                        goal_stiffnesses,
                    },
                )
            })
            .collect();

        let mut non_manifold = None;
        let collider_input = collider_inputs
            .iter()
            .map(|(name, input)| {
                let vertex_positions: Vec<_> = input
                    .vertex_positions
                    .chunks_exact(3)
                    .map(Vector3::from_column_slice)
                    .collect();
                let vertex_velocities = vec![Vector3::zeros(); vertex_positions.len()];
                let triangles: Vec<_> = input
                    .triangles
                    .chunks_exact(3)
                    .map(|chunk| [chunk[0] as u32, chunk[1] as u32, chunk[2] as u32])
                    .collect();

                let mut vertex_neighbors = vec![None; vertex_positions.len()];
                let order_edge = |[a, b]: [u32; 2]| if a < b { [a, b] } else { [b, a] };
                let mut edges_with_opposites: FxHashMap<[u32; 2], (u32, Option<u32>)> =
                    Default::default();
                for &[a, b, c] in &triangles {
                    vertex_neighbors[a as usize].get_or_insert([b, c]);
                    vertex_neighbors[b as usize].get_or_insert([c, a]);
                    vertex_neighbors[c as usize].get_or_insert([a, b]);

                    for (edge, opposite) in [[a, b], [b, c], [c, a]]
                        .into_iter()
                        .map(order_edge)
                        .zip([c, a, b].into_iter())
                    {
                        match edges_with_opposites.entry(edge) {
                            Entry::Occupied(mut occupied_entry) => {
                                let opposites = occupied_entry.get_mut();
                                if opposites.1.replace(opposite).is_some() {
                                    non_manifold = Some(edge)
                                }
                            }
                            Entry::Vacant(vacant_entry) => {
                                vacant_entry.insert((opposite, None));
                            }
                        }
                    }
                }
                let edges_with_opposites: FxHashMap<[u32; 2], [u32; 2]> = edges_with_opposites
                    .into_iter()
                    .filter_map(|(key, (first, second))| {
                        second.map(|second| (key, [first, second]))
                    })
                    .collect();

                let vertex_normals = vertex_neighbors
                    .into_iter()
                    .enumerate()
                    .map(|(this, neighbors)| {
                        let [mut neighbor, mut next_neighbor] = neighbors?;

                        let a = &vertex_positions[this];
                        let mut b = &vertex_positions[neighbor as usize];
                        let mut c = &vertex_positions[next_neighbor as usize];

                        let start = neighbor;
                        let mut normal = Vector3::zeros();
                        while start != next_neighbor {
                            let opposites = edges_with_opposites
                                .get(&order_edge([this as u32, next_neighbor]))?;

                            normal += (b - a).cross(&(c - a));

                            if opposites[0] != neighbor {
                                neighbor = next_neighbor;
                                next_neighbor = opposites[0];
                            } else {
                                neighbor = next_neighbor;
                                next_neighbor = opposites[1];
                            };

                            b = c;
                            c = &vertex_positions[next_neighbor as usize];
                        }

                        Unit::try_new(normal, NORMALIZATION_EPS)
                    })
                    .collect();

                (
                    name.clone(),
                    InterpolatedInputCollider {
                        vertex_positions,
                        vertex_normals,
                        vertex_velocities,
                        triangles,
                        edges_with_opposites,
                    },
                )
            })
            .collect();

        if let Some([a, b]) = non_manifold {
            bail!("Non manifold edge: {a}, {b}")
        }

        Ok(Self {
            gravity,
            particles_input,
            collider_input,
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InterpolatedInputParticles {
    pub goal_positions: Vec<Vector3<T>>,
    pub goal_stiffnesses: Vec<T>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InterpolatedInputCollider {
    pub vertex_positions: Vec<Vector3<T>>,
    pub vertex_normals: Vec<Option<Unit<Vector3<T>>>>,
    pub vertex_velocities: Vec<Vector3<T>>,
    pub triangles: Vec<[u32; 3]>,
    pub edges_with_opposites: FxHashMap<[u32; 2], [u32; 2]>,
}

pub struct InputInterpolationPoint {
    pub frame: usize,
    pub interpolant: InterpolatedInput,
}

pub struct InputInterpolation {
    input_reader: InputReader,
    a: Option<InputInterpolationPoint>,
    b: Option<InputInterpolationPoint>,
}

impl InputInterpolation {
    pub fn new(input_reader: InputReader) -> Result<Self> {
        ensure!(input_reader.len() > 0);
        Ok(Self {
            input_reader,
            a: None,
            b: None,
        })
    }

    pub fn load(&mut self, frame: usize) -> Result<()> {
        profile!("load interpolants");

        let max_frame = self.input_reader.len() - 1;

        // if we're too far, just use the last available and skip b
        let frame = frame.min(max_frame);

        // a already corret, so b must be as well (could be none)
        if self.a.as_ref().is_some_and(|point| point.frame == frame) {
            return Ok(());
        }

        // always load something into a
        // could be that we already have the next in b
        if let Some(b) = self.b.take()
            && b.frame == frame
        {
            self.a = Some(b);
        } else {
            let input_frame = self.input_reader.read_frame(frame)?;
            let interpolant = InterpolatedInput::new(input_frame)?;
            self.a = Some(InputInterpolationPoint { frame, interpolant })
        }

        // might skip b
        if frame == max_frame {
            return Ok(());
        }

        // load b
        let frame = frame + 1;
        let input_frame = self.input_reader.read_frame(frame)?;
        let interpolant = InterpolatedInput::new(input_frame)?;
        self.b = Some(InputInterpolationPoint { frame, interpolant });

        Ok(())
    }

    pub fn a(&self) -> Option<&InputInterpolationPoint> {
        self.a.as_ref()
    }

    pub fn b(&self) -> Option<&InputInterpolationPoint> {
        self.b.as_ref()
    }
}
