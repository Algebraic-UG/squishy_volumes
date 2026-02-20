// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::iter::empty;
use thiserror::Error;

use nalgebra::{Matrix4, Vector3};
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    math::flat::{Flat3, Flat9, Flat16},
    state::{ObjectIndex, grids::GridMomentum, particles::ParticleState},
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
    GridMomentums(AttributeGridMomentums),
    GridColliderDistance(AttributeGridColliderDistance),
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
pub enum AttributeGridColliderDistance {
    Positions,
    ColliderDistances(usize),
    ColliderDistanceNormals(usize),
}

#[derive(Serialize, Deserialize)]
pub enum AttributeGridMomentums {
    Free(AttributeGridMomentum),
    Conformed {
        name: String,
        attribute: AttributeGridMomentum,
    },
}

#[derive(EnumIter, Serialize, Deserialize)]
pub enum AttributeGridMomentum {
    Masses,
    Positions,
    Velocities,
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
    Transformations,
    ColliderInsides(usize),
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
            .chain(AttributeGridColliderDistance::iter().map(Attribute::GridColliderDistance))
            .chain(
                AttributeGridMomentum::iter()
                    .map(AttributeGridMomentums::Free)
                    .map(Attribute::GridMomentums),
            )
            .chain(self.name_map.iter().flat_map(|(name, _)| {
                AttributeMesh::iter().map(|attribute| Attribute::Mesh {
                    name: name.to_string(),
                    attribute,
                })
            }))
    }

    pub fn fetch_flat_attribute(
        &self,
        grid_node_size: T,
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
                            AttributeParticles::Positions => {
                                is.flat_map(|i| ps.positions[i].flat()).collect()
                            }
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
                            AttributeParticles::Transformations => is
                                .flat_map(|i| {
                                    let position_gradient = &ps.position_gradients[i];
                                    let scale = ps.initial_volumes[i].powf(1. / 3.);
                                    Matrix4::from_columns(&[
                                        position_gradient.column(0).scale(scale).push(0.),
                                        position_gradient.column(1).scale(scale).push(0.),
                                        position_gradient.column(2).scale(scale).push(0.),
                                        ps.positions[i].push(1.),
                                    ])
                                    .flat()
                                })
                                .collect(),
                            AttributeParticles::ColliderInsides(collider_idx) => is
                                .map(|i| {
                                    ps.collider_insides[i]
                                        .get(&collider_idx)
                                        .map(|inside| if *inside { -1. } else { 1. })
                                        .unwrap_or(0.)
                                })
                                .collect(),
                        }
                    }
                    _ => Err(AttributeError::ObjectTypeMismatch(name.clone()))?,
                }
            }
            Attribute::GridColliderDistance(attribute) => match attribute {
                AttributeGridColliderDistance::Positions => self
                    .grid_collider
                    .keys()
                    .map(|grid_node_idx| grid_node_idx.map(|i| i as T) * grid_node_size)
                    .flat_map(|position| position.flat())
                    .collect(),
                AttributeGridColliderDistance::ColliderDistances(collider_idx) => self
                    .grid_collider
                    .values()
                    .map(|grid_node| {
                        grid_node
                            .infos
                            .get(&collider_idx)
                            .map(|info| info.distance)
                            .unwrap_or(T::MAX)
                    })
                    .collect(),
                AttributeGridColliderDistance::ColliderDistanceNormals(collider_idx) => self
                    .grid_collider
                    .values()
                    .flat_map(|grid_node| {
                        grid_node
                            .infos
                            .get(&collider_idx)
                            .map(|weighted_distance| weighted_distance.normal)
                            .unwrap_or(Vector3::zeros())
                            .flat()
                    })
                    .collect(),
            },
            Attribute::GridMomentums(attribute) => {
                let fetch_flat_attribute_grid_momentum =
                    |grid: &GridMomentum, attribute: AttributeGridMomentum| match attribute {
                        AttributeGridMomentum::Masses => grid.masses.clone(),
                        AttributeGridMomentum::Positions => {
                            // after deserializing the hashmap has a different iteration order
                            let mut messed_up = grid.map.iter().collect::<Vec<_>>();
                            messed_up.sort_unstable_by_key(|(_, i)| **i);
                            messed_up
                                .into_iter()
                                .map(|(grid_node_idx, _)| {
                                    grid_node_idx.map(|i| i as T) * grid_node_size
                                })
                                .flat_map(|position| position.flat())
                                .collect()
                        }
                        AttributeGridMomentum::Velocities => {
                            grid.velocities.iter().flat_map(Flat3::flat).collect()
                        }
                    };
                match attribute {
                    AttributeGridMomentums::Free(attribute) => {
                        fetch_flat_attribute_grid_momentum(&self.grid_momentum, attribute)
                    }
                    AttributeGridMomentums::Conformed { name, attribute } => {
                        let object_idx = self
                            .name_map
                            .get(&name)
                            .ok_or(AttributeError::ObjectMissing(name.clone()))?;
                        let ObjectIndex::Collider(collider_idx) = object_idx else {
                            Err(AttributeError::ObjectTypeMismatch(name.clone()))?
                        };
                        fetch_flat_attribute_grid_momentum(
                            &self.grid_collider_momentums[*collider_idx],
                            attribute,
                        )
                    }
                }
            }
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
