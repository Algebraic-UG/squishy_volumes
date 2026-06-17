// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use squishy_volumes_util::{Flat3, Flat9 as _, Flat16 as _};
use std::iter::empty;
use thiserror::Error;

use nalgebra::Matrix4;
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    input_file::InputConsts,
    state::{ObjectIndex, grids::GridKey, particles::ParticleState},
};

use super::State;

#[derive(Error, Debug)]
pub enum AttributeError {
    #[error("The object is missing: {0}")]
    ObjectMissing(String),
    #[error("The object's type doesn't match: {0}")]
    ObjectTypeMismatch(String),
}

#[derive(Serialize, Deserialize)]
pub enum Attribute {
    Const(AttributeConst),
    Object {
        name: String,
        attribute: AttributeObject,
    },
    Mesh {
        name: String,
        attribute: AttributeMesh,
    },
    Grid(AttributeGrid),
}

#[derive(Default, EnumIter, Serialize, Deserialize)]
pub enum AttributeConst {
    #[default]
    GridNodeSize,
    FramesPerSecond,
    SimulationScale,
    DomainMin,
    DomainMax,
}

#[derive(EnumIter, Serialize, Deserialize)]
pub enum AttributeGrid {
    Masses,
    Positions,
    Velocities,
    ColliderBitsA,
    ColliderBitsB,
}

#[derive(Serialize, Deserialize)]
pub enum AttributeObject {
    Particles(AttributeParticles),
    Collider(AttributeCollider),
}

#[derive(EnumIter, Serialize, Deserialize)]
pub enum AttributeParticles {
    States,
    Masses,
    InitialVolumes,
    Positions,
    InitialPositions,
    Velocities,
    PositionGradients,
    ElasticEnergies,
    Sizes,
    Transformations,
    ColliderBitsA,
    ColliderBitsB,
}

#[derive(EnumIter, Serialize, Deserialize)]
pub enum AttributeCollider {
    Samples,
    SampleNormals,
    SampleVelocities,
    Transformation,
}

#[derive(EnumIter, Serialize, Deserialize)]
pub enum AttributeMesh {
    Vertices,
    Triangles,
    Scale,
    Position,
    Orientation,
}

impl State {
    pub fn available_attributes(&self) -> impl Iterator<Item = Attribute> + '_ {
        empty()
            .chain(AttributeConst::iter().map(Attribute::Const))
            .chain(AttributeGrid::iter().map(Attribute::Grid))
            .chain(self.name_map.iter().flat_map(|(name, _)| {
                AttributeMesh::iter().map(|attribute| Attribute::Mesh {
                    name: name.to_string(),
                    attribute,
                })
            }))
    }

    pub fn fetch_flat_attribute(
        &self,
        consts: &InputConsts,
        attribute: Attribute,
    ) -> Result<Vec<T>, AttributeError> {
        let flat_attribute = match attribute {
            Attribute::Object { name, attribute } => {
                let object_idx = self
                    .name_map
                    .get(&name)
                    .ok_or(AttributeError::ObjectMissing(name.clone()))?;
                match (attribute, object_idx) {
                    (AttributeObject::Particles(attribute), ObjectIndex::Particles(idx)) => {
                        let ps = &self.particles;
                        let is = self.particle_objects[*idx]
                            .particles
                            .iter()
                            .map(|idx| self.particles.reverse_sort_map[*idx]);
                        match attribute {
                            AttributeParticles::States => {
                                is.map(|i| ps.states[i].to_float()).collect()
                            }
                            AttributeParticles::Masses => is.map(|i| ps.masses[i]).collect(),
                            AttributeParticles::InitialVolumes => {
                                is.map(|i| ps.initial_volumes[i]).collect()
                            }
                            AttributeParticles::Positions => is
                                .flat_map(|i| ps.positions[i].scale(consts.simulation_scale).flat())
                                .collect(),
                            AttributeParticles::InitialPositions => {
                                is.flat_map(|i| ps.initial_positions[i].flat()).collect()
                            }
                            AttributeParticles::Velocities => {
                                is.flat_map(|i| ps.velocities[i].flat()).collect()
                            }
                            AttributeParticles::PositionGradients => {
                                is.flat_map(|i| ps.position_gradients[i].flat()).collect()
                            }
                            AttributeParticles::ElasticEnergies => {
                                is.map(|i| ps.elastic_energies[i]).collect()
                            }
                            AttributeParticles::Sizes => is
                                .map(|i| {
                                    ps.initial_volumes[i].powf(1. / 3.) * consts.simulation_scale
                                })
                                .collect(),
                            AttributeParticles::Transformations => is
                                .flat_map(|i| {
                                    let position_gradient = &ps.position_gradients[i];
                                    Matrix4::from_columns(&[
                                        position_gradient.column(0).push(0.),
                                        position_gradient.column(1).push(0.),
                                        position_gradient.column(2).push(0.),
                                        ps.positions[i].scale(consts.simulation_scale).push(1.),
                                    ])
                                    .flat()
                                })
                                .collect(),
                            AttributeParticles::ColliderBitsA => {
                                is.map(|i| (ps.collider_bits[i] & 0xFFFF) as f32).collect()
                            }
                            AttributeParticles::ColliderBitsB => {
                                is.map(|i| (ps.collider_bits[i] >> 16) as f32).collect()
                            }
                        }
                    }
                    _ => Err(AttributeError::ObjectTypeMismatch(name.clone()))?,
                }
            }
            Attribute::Grid(attribute) => match attribute {
                AttributeGrid::Masses => self.grid.masses.clone(),
                AttributeGrid::Positions => self
                    .grid
                    .map
                    .keys()
                    .flat_map(|GridKey { node_id, .. }| {
                        node_id
                            .map(|c| c as f32 * consts.unscaled_grid_node_size())
                            .flat()
                    })
                    .collect(),
                AttributeGrid::Velocities => self
                    .grid
                    .map
                    .values()
                    .flat_map(|index| self.grid.velocities[*index as usize].flat())
                    .collect(),
                AttributeGrid::ColliderBitsA => self
                    .grid
                    .map
                    .keys()
                    .map(|GridKey { collider_bits, .. }| (collider_bits & 0xFFFF) as f32)
                    .collect(),
                AttributeGrid::ColliderBitsB => self
                    .grid
                    .map
                    .keys()
                    .map(|GridKey { collider_bits, .. }| (collider_bits >> 16) as f32)
                    .collect(),
            },
            _ => unreachable!("Should have been handled before"),
        };
        Ok(flat_attribute)
    }
}

impl ParticleState {
    fn to_float(&self) -> T {
        match self {
            Self::Active => 0.,
            Self::Tombstoned => 1.,
        }
    }
}
