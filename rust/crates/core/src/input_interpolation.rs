// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use thiserror::Error;

use std::mem::take;

use nalgebra::Vector3;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use squishy_volumes_api::T;
use squishy_volumes_util::{
    Aabb, BoundingVolumeHierarchy,
    triangle::{Opposites, Triangle},
};

use crate::{
    FrameBulkCollider, FrameBulkParticles, ParticleFlags,
    input_file::{InputConsts, InputError, InputFrame, InputReader},
    profile,
};

#[derive(Error, Debug)]
pub enum InterpolationError {
    #[error("No frames to interpolate.")]
    NoFrames,

    #[error("'{name}': length mismatch between '{attribute_a}' and '{attribute_b}'")]
    AttributeLengthMismatch {
        name: String,
        attribute_a: String,
        attribute_b: String,
    },

    #[error("'{name}': flattened '{attribute}' is not multiple of {multiple}")]
    FlattedNotCorrectMultiple {
        name: String,
        attribute: String,
        multiple: usize,
    },

    #[error("'{name}': non-manifold edge between {vertex_index_a} and {vertex_index_b}")]
    NonManifoldEdge {
        name: String,
        vertex_index_a: u32,
        vertex_index_b: u32,
    },

    #[error("'{name}': vertex index out of range in triangle {triangle_index}")]
    VertexIndexOutOfRange { name: String, triangle_index: usize },

    #[error("Failed to read input: {0}")]
    InputError(#[from] InputError),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InterpolatedInput {
    pub gravity: Vector3<T>,

    pub particle_flags: Vec<ParticleFlags>,
    pub particle_goal_positions: Vec<Vector3<T>>,

    pub vertex_positions: Vec<Vector3<T>>,
    pub vertex_normals: Vec<Vector3<T>>,

    pub triangle_frictions: Vec<T>,
    pub triangle_normals: Vec<Vector3<T>>,
}

#[derive(Default)]
pub struct InputInterpolationPoint {
    pub frame: usize,

    pub gravity: Vector3<T>,

    pub particle_flags: Vec<ParticleFlags>,
    pub particle_goal_positions: Vec<Vector3<T>>,

    pub vertex_positions: Vec<Vector3<T>>,
    pub triangle_frictions: Vec<T>,
}

impl InputInterpolationPoint {
    fn new(
        consts: &InputConsts,
        frame: usize,
        InputFrame {
            gravity,
            particles_inputs,
            collider_inputs,
        }: InputFrame,
    ) -> Result<Self, InterpolationError> {
        let mut particle_flags: Vec<ParticleFlags> = Default::default();
        let mut particle_goal_positions: Vec<Vector3<T>> = Default::default();
        for (name, mut input) in particles_inputs.into_iter() {
            if !input.goal_positions.len().is_multiple_of(3) {
                tracing::error!(goal_positions_len = input.goal_positions.len());
                return Err(InterpolationError::FlattedNotCorrectMultiple {
                    name,
                    attribute: format!("{:?}", FrameBulkParticles::GoalPositions),
                    multiple: 3,
                });
            }

            if !input.goal_positions.is_empty()
                && input.flags.len() != input.goal_positions.len() / 3
            {
                tracing::error!(
                    flags_len = input.flags.len(),
                    goal_positions_len = input.goal_positions.len()
                );
                return Err(InterpolationError::AttributeLengthMismatch {
                    name,
                    attribute_a: "ParticleFlags".to_string(),
                    attribute_b: format!("{:?}", FrameBulkParticles::GoalPositions),
                });
            }

            particle_flags.append(&mut input.flags);
            particle_goal_positions.extend(input.goal_positions.chunks_exact(3).map(|chunk| {
                Vector3::from_column_slice(chunk).scale(1. / consts.simulation_scale)
            }));
        }

        let mut vertex_positions: Vec<Vector3<T>> = Default::default();
        let mut triangle_frictions: Vec<T> = Default::default();
        for (name, mut input) in collider_inputs.into_iter() {
            if !input.vertex_positions.len().is_multiple_of(3) {
                return Err(InterpolationError::FlattedNotCorrectMultiple {
                    name,
                    attribute: format!("{:?}", FrameBulkCollider::VertexPositions),
                    multiple: 3,
                });
            }
            vertex_positions.extend(input.vertex_positions.chunks_exact(3).map(|chunk| {
                Vector3::from_column_slice(chunk).scale(1. / consts.simulation_scale)
            }));
            triangle_frictions.append(&mut input.triangle_frictions);
        }

        Ok(Self {
            frame,
            gravity,
            particle_flags,
            particle_goal_positions,
            vertex_positions,
            triangle_frictions,
        })
    }
}

pub struct Topology {
    pub vertex_triangle_lists: Vec<SmallVec<[u32; 8]>>,
    pub triangle_indices: Vec<Triangle>,
    pub triangle_opposites: Vec<Opposites>,
    pub triangle_collider: Vec<u32>,
}

pub struct InputInterpolation {
    input_reader: InputReader,

    topology: Topology,

    // needs to be rebuilt every frame change
    bvh: BoundingVolumeHierarchy,

    // b could be none (end of input)
    a: InputInterpolationPoint,
    b: Option<InputInterpolationPoint>,
}

impl InputInterpolation {
    pub fn new(
        mut input_reader: InputReader,
        consts: &InputConsts,
        frame: usize,
    ) -> Result<Self, InterpolationError> {
        if input_reader.is_empty() {
            return Err(InterpolationError::NoFrames);
        }

        let mut a = Default::default();
        let mut b = Default::default();
        load_points(&mut input_reader, &mut a, &mut b, consts, frame)?;
        let a = a.expect("a missing");

        let topology = Topology::new(&mut input_reader)?;
        let bvh = update_bvh(consts, &topology, &a, b.as_ref());

        Ok(Self {
            input_reader,
            topology,
            bvh,
            a,
            b,
        })
    }

    pub fn load(&mut self, consts: &InputConsts, frame: usize) -> Result<(), InterpolationError> {
        let prior_frame = self.a.frame;

        // weird little dance s.t. the type of a can be non-option
        let mut a = Some(take(&mut self.a));
        load_points(&mut self.input_reader, &mut a, &mut self.b, consts, frame)?;
        self.a = a.expect("a missing");

        if prior_frame != self.a.frame {
            self.bvh = update_bvh(consts, &self.topology, &self.a, self.b.as_ref());
        }

        Ok(())
    }

    pub fn topology(&self) -> &Topology {
        &self.topology
    }

    pub fn bvh(&self) -> &BoundingVolumeHierarchy {
        &self.bvh
    }

    pub fn a(&self) -> &InputInterpolationPoint {
        &self.a
    }

    pub fn b(&self) -> Option<&InputInterpolationPoint> {
        self.b.as_ref()
    }
}

// after this, a is always Some
fn load_points(
    input_reader: &mut InputReader,
    a: &mut Option<InputInterpolationPoint>,
    b: &mut Option<InputInterpolationPoint>,
    consts: &InputConsts,
    frame: usize,
) -> Result<(), InterpolationError> {
    profile!("load interpolants");

    let max_frame = input_reader.len() - 1;

    // if we're too far, just use the last available and skip b
    let frame = frame.min(max_frame);

    // a already correct, so b must be as well (could be none)
    if a.as_ref().is_some_and(|point| point.frame == frame) {
        return Ok(());
    }

    // always load something into a
    // could be that we already have the next in b
    if let Some(b) = b.take()
        && b.frame == frame
    {
        *a = Some(b);
    } else {
        let input_frame = input_reader.read_frame(frame)?;
        *a = Some(InputInterpolationPoint::new(consts, frame, input_frame)?);
    }

    // might skip b
    if frame == max_frame {
        return Ok(());
    }

    // load b
    let frame = frame + 1;
    let input_frame = input_reader.read_frame(frame)?;
    *b = Some(InputInterpolationPoint::new(consts, frame, input_frame)?);

    Ok(())
}

impl Topology {
    fn new(input_reader: &mut InputReader) -> Result<Self, InterpolationError> {
        profile!("load topology");

        let frame = input_reader.read_frame(0).expect("no frames");

        let mut vertex_index_offset: u32 = 0;
        let mut triangle_indices: Vec<Triangle> = Default::default();
        let mut triangle_collider: Vec<u32> = Default::default();
        let mut triangle_opposites: Vec<Opposites> = Default::default();
        for (collider, (name, input)) in frame.collider_inputs.into_iter().enumerate() {
            if !input.triangles.len().is_multiple_of(3) {
                tracing::error!(triangle_len = input.triangles.len());
                return Err(InterpolationError::FlattedNotCorrectMultiple {
                    name,
                    attribute: format!("{:?}", FrameBulkCollider::Triangles),
                    multiple: 3,
                });
            }

            if !input.vertex_positions.len().is_multiple_of(3) {
                tracing::error!(vertex_positions_len = input.vertex_positions.len());
                return Err(InterpolationError::FlattedNotCorrectMultiple {
                    name,
                    attribute: format!("{:?}", FrameBulkCollider::VertexPositions),
                    multiple: 3,
                });
            }

            for (triangle_index, &vertex_index) in input.triangles.iter().enumerate() {
                if vertex_index < 0 || vertex_index as usize >= input.vertex_positions.len() / 3 {
                    return Err(InterpolationError::VertexIndexOutOfRange {
                        name,
                        triangle_index: triangle_index / 3,
                    });
                }
            }

            let local_triangle_indices = input
                .triangles
                .chunks_exact(3)
                .map(|chunk| Triangle {
                    a: chunk[0] as u32,
                    b: chunk[1] as u32,
                    c: chunk[2] as u32,
                })
                .collect::<Vec<_>>();

            let order_edge = |[a, b]: [u32; 2]| if a < b { [a, b] } else { [b, a] };
            let mut edge_to_triangle: FxHashMap<[u32; 2], SmallVec<[u32; 2]>> = Default::default();
            for (index, indices) in local_triangle_indices.iter().enumerate() {
                for (a, b) in indices.into_iter().zip(indices.into_iter().cycle().skip(1)) {
                    edge_to_triangle
                        .entry(order_edge([a, b]))
                        .or_default()
                        .push(index as u32);
                }
            }

            for (&[vertex_index_a, vertex_index_b], triangles) in edge_to_triangle.iter() {
                if triangles.len() > 2 {
                    return Err(InterpolationError::NonManifoldEdge {
                        name,
                        vertex_index_a,
                        vertex_index_b,
                    });
                }
            }

            let triangle_index_offset = triangle_indices.len() as u32;

            triangle_opposites.extend(local_triangle_indices.iter().enumerate().map(
                |(index, indices)| -> Opposites {
                    indices
                        .into_iter()
                        .zip(indices.into_iter().cycle().skip(1))
                        .map(|(a, b)| -> u32 {
                            edge_to_triangle
                                .get(&order_edge([a, b]))
                                .unwrap()
                                .iter()
                                .cloned()
                                .find(|&other| other != index as u32)
                                .map(|triangle_index| triangle_index + triangle_index_offset)
                                .unwrap_or(u32::MAX)
                        })
                        .into()
                },
            ));
            triangle_indices.extend(local_triangle_indices.into_iter().map(
                |Triangle { a, b, c }| Triangle {
                    a: a + vertex_index_offset,
                    b: b + vertex_index_offset,
                    c: c + vertex_index_offset,
                },
            ));
            triangle_collider.resize(
                triangle_collider.len() + input.triangles.len(),
                collider as u32,
            );

            vertex_index_offset += input.vertex_positions.len() as u32 / 3;
        }

        let mut vertex_triangle_lists: Vec<SmallVec<[u32; 8]>> =
            vec![Default::default(); vertex_index_offset as usize];
        for (triangle_index, indices) in triangle_indices.iter().enumerate() {
            for vertex_index in indices.iter() {
                vertex_triangle_lists[*vertex_index as usize].push(triangle_index as u32);
            }
        }
        vertex_triangle_lists
            .iter_mut()
            .enumerate()
            .for_each(|(this_vertex, triangles)| {
                let mut neighbor_counts: FxHashMap<u32, u8> = Default::default();
                for triangle_index in triangles.iter() {
                    for &vertex_index in triangle_indices[*triangle_index as usize].iter() {
                        if vertex_index != this_vertex as u32 {
                            *neighbor_counts.entry(vertex_index).or_default() += 1;
                        }
                    }
                }
                assert!(
                    neighbor_counts.values().all(|&count| count <= 2),
                    "missed non-manifoldness before"
                );
                if neighbor_counts.into_values().any(|count| count != 2) {
                    triangles.clear();
                }
            });

        Ok(Self {
            vertex_triangle_lists,
            triangle_indices,
            triangle_opposites,
            triangle_collider,
        })
    }
}

fn update_bvh(
    consts: &InputConsts,
    topology: &Topology,
    a: &InputInterpolationPoint,
    b: Option<&InputInterpolationPoint>,
) -> BoundingVolumeHierarchy {
    profile!("update bvh");

    let margin = consts.forget_distance();
    let aabbs = topology
        .triangle_indices
        .iter()
        .map(|triangle| {
            let aabb = if let Some(b) = b {
                Aabb::new_from_ref(triangle.iter().flat_map(|vertex_index| {
                    [
                        &a.vertex_positions[*vertex_index as usize],
                        &b.vertex_positions[*vertex_index as usize],
                    ]
                }))
            } else {
                Aabb::new_from_ref(
                    triangle
                        .iter()
                        .map(|vertex_index| &a.vertex_positions[*vertex_index as usize]),
                )
            };

            Aabb {
                min: aabb
                    .min
                    .map(|c| ((c - margin) / consts.leaf_size).floor() as i32),
                max: aabb
                    .max
                    .map(|c| ((c + margin) / consts.leaf_size).ceil() as i32),
            }
        })
        .collect();

    BoundingVolumeHierarchy::new(aabbs, consts.leaf_threshold)
}
