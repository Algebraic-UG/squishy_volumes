// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::iter::empty;

use anyhow::{Context, Result, bail};
use iter_enumeration::IntoIterEnum3;
use nalgebra::{Matrix4, Vector3};
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    math::flat::{Flat3, Flat9, Flat16},
    simulation::{grids::GridMomentum, particles::ParticleState},
};

use super::{ObjectIndex, State};

#[derive(Serialize, Deserialize)]
pub enum Attribute {
    Setting(AttributeSetting),
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

#[derive(EnumIter, Serialize, Deserialize)]
pub enum AttributeSetting {
    GridNodeSize,
    ParticleSize,
    FramesPerSecond,
    Gravity,
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
    Solid(AttributeSolid),
    Fluid(AttributeFluid),
    Collider(AttributeCollider),
}

#[derive(EnumIter, Serialize, Deserialize)]
pub enum AttributeSolid {
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
pub enum AttributeFluid {
    States,
    Positions,
    InitialPositions,
    Velocities,
    Transformations,
    ColliderInsides(usize),
    ElasticEnergies,
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
            .chain(AttributeSetting::iter().map(Attribute::Setting))
            .chain(AttributeGridColliderDistance::iter().map(Attribute::GridColliderDistance))
            .chain(
                AttributeGridMomentum::iter()
                    .map(AttributeGridMomentums::Free)
                    .map(Attribute::GridMomentums),
            )
            .chain(
                self.name_map
                    .iter()
                    .filter_map(|(name, object_idx)| {
                        (matches!(object_idx, ObjectIndex::Collider(_))).then_some(name.clone())
                    })
                    .flat_map(|name| {
                        AttributeGridMomentum::iter().map(move |attribute| {
                            AttributeGridMomentums::Conformed {
                                name: name.clone(),
                                attribute,
                            }
                        })
                    })
                    .map(Attribute::GridMomentums),
            )
            .chain(self.name_map.iter().flat_map(|(name, object_idx)| {
                match object_idx {
                    ObjectIndex::Solid(_) => AttributeSolid::iter()
                        .map(AttributeObject::Solid)
                        .iter_enum_3a(),
                    ObjectIndex::Fluid(_) => AttributeFluid::iter()
                        .map(AttributeObject::Fluid)
                        .iter_enum_3b(),
                    ObjectIndex::Collider(_) => AttributeCollider::iter()
                        .map(AttributeObject::Collider)
                        .iter_enum_3c(),
                }
                .map(|attribute| Attribute::Object {
                    name: name.to_string(),
                    attribute,
                })
            }))
            .chain(self.name_map.iter().flat_map(|(name, _)| {
                AttributeMesh::iter().map(|attribute| Attribute::Mesh {
                    name: name.to_string(),
                    attribute,
                })
            }))
    }

    pub fn fetch_flat_attribute(&self, grid_node_size: T, attribute: Attribute) -> Result<Vec<T>> {
        let flat_attribute = match attribute {
            Attribute::Setting(_) => bail!("attribute should have been handled before"),
            Attribute::Mesh { .. } => bail!("attribute should have been handled before"),
            Attribute::Object { name, attribute } => {
                let object_idx = self.name_map.get(&name).context("Missing object")?;
                match (attribute, object_idx) {
                    (AttributeObject::Solid(attribute), ObjectIndex::Solid(idx)) => {
                        let solid = &self.solid_objects[*idx];
                        let ps = &self.particles;
                        let is = solid
                            .particles
                            .iter()
                            .map(|idx| self.particles.reverse_sort_map[*idx]);
                        match attribute {
                            AttributeSolid::States => is.map(|i| ps.states[i].to_float()).collect(),
                            AttributeSolid::Masses => is.map(|i| ps.masses[i]).collect(),
                            AttributeSolid::InitialVolumes => {
                                is.map(|i| ps.initial_volumes[i]).collect()
                            }
                            AttributeSolid::Positions => {
                                is.flat_map(|i| ps.positions[i].flat()).collect()
                            }
                            AttributeSolid::InitialPositions => {
                                is.flat_map(|i| ps.initial_positions[i].flat()).collect()
                            }
                            AttributeSolid::Velocities => {
                                is.flat_map(|i| ps.velocities[i].flat()).collect()
                            }
                            AttributeSolid::PositionGradients => {
                                is.flat_map(|i| ps.position_gradients[i].flat()).collect()
                            }
                            AttributeSolid::ElasticEnergies => {
                                is.map(|i| ps.elastic_energies[i]).collect()
                            }
                            AttributeSolid::Transformations => is
                                .flat_map(|i| {
                                    let position_gradient = &ps.position_gradients[i];
                                    Matrix4::from_columns(&[
                                        position_gradient.column(0).push(0.),
                                        position_gradient.column(1).push(0.),
                                        position_gradient.column(2).push(0.),
                                        ps.positions[i].push(1.),
                                    ])
                                    .flat()
                                })
                                .collect(),
                            AttributeSolid::ColliderInsides(collider_idx) => is
                                .map(|i| {
                                    ps.collider_insides[i]
                                        .get(&collider_idx)
                                        .map(|inside| if *inside { -1. } else { 1. })
                                        .unwrap_or(0.)
                                })
                                .collect(),
                        }
                    }
                    (AttributeObject::Fluid(attribute), ObjectIndex::Fluid(idx)) => {
                        let fluid = &self.fluid_objects[*idx];
                        let ps = &self.particles;
                        let is = fluid
                            .particles
                            .iter()
                            .map(|idx| self.particles.reverse_sort_map[*idx]);
                        match attribute {
                            AttributeFluid::States => is.map(|i| ps.states[i].to_float()).collect(),
                            AttributeFluid::Positions => {
                                is.flat_map(|i| ps.positions[i].flat()).collect()
                            }
                            AttributeFluid::InitialPositions => {
                                is.flat_map(|i| ps.initial_positions[i].flat()).collect()
                            }
                            AttributeFluid::Velocities => {
                                is.flat_map(|i| ps.velocities[i].flat()).collect()
                            }
                            AttributeFluid::Transformations => is
                                .flat_map(|i| {
                                    let position_gradient = &ps.position_gradients[i];
                                    Matrix4::from_columns(&[
                                        position_gradient.column(0).push(0.),
                                        position_gradient.column(1).push(0.),
                                        position_gradient.column(2).push(0.),
                                        ps.positions[i].push(1.),
                                    ])
                                    .flat()
                                })
                                .collect(),
                            AttributeFluid::ColliderInsides(collider_idx) => is
                                .map(|i| {
                                    ps.collider_insides[i]
                                        .get(&collider_idx)
                                        .map(|inside| if *inside { -1. } else { 1. })
                                        .unwrap_or(0.)
                                })
                                .collect(),
                            AttributeFluid::ElasticEnergies => {
                                is.map(|i| ps.elastic_energies[i]).collect()
                            }
                        }
                    }
                    (AttributeObject::Collider(attribute), ObjectIndex::Collider(object_idx)) => {
                        let collider = &self.collider_objects[*object_idx];
                        match attribute {
                            AttributeCollider::Samples => collider
                                .surface_samples
                                .iter()
                                .flat_map(|s| {
                                    collider.kinematic.to_world_position(s.position).flat()
                                })
                                .collect(),
                            AttributeCollider::SampleNormals => collider
                                .surface_samples
                                .iter()
                                .flat_map(|s| collider.kinematic.to_world_normal(s.normal).flat())
                                .collect(),
                            AttributeCollider::SampleVelocities => collider
                                .surface_samples
                                .iter()
                                .flat_map(|s| {
                                    collider
                                        .kinematic
                                        .point_velocity_from_local(s.position)
                                        .flat()
                                })
                                .collect(),
                            AttributeCollider::Transformation => {
                                collider.kinematic.transformation().flat().into()
                            }
                        }
                    }
                    _ => bail!("Object type missmatch: '{name}'"),
                }
            }
            Attribute::GridColliderDistance(attribute) => match attribute {
                AttributeGridColliderDistance::Positions => self
                    .grid_collider_distances
                    .keys()
                    .map(|grid_node_idx| grid_node_idx.map(|i| i as T) * grid_node_size)
                    .flat_map(|position| position.flat())
                    .collect(),
                AttributeGridColliderDistance::ColliderDistances(collider_idx) => self
                    .grid_collider_distances
                    .values()
                    .map(|grid_node| {
                        grid_node
                            .try_lock()
                            .unwrap()
                            .weighted_distances
                            .get(&collider_idx)
                            .map(|weighted_distance| weighted_distance.distance)
                            .unwrap_or(T::MAX)
                    })
                    .collect(),
                AttributeGridColliderDistance::ColliderDistanceNormals(collider_idx) => self
                    .grid_collider_distances
                    .values()
                    .flat_map(|grid_node| {
                        grid_node
                            .try_lock()
                            .unwrap()
                            .weighted_distances
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
                        let object_idx = self.name_map.get(&name).context("Missing object")?;
                        let ObjectIndex::Collider(collider_idx) = object_idx else {
                            bail!("Object type missmatch");
                        };
                        fetch_flat_attribute_grid_momentum(
                            &self.grid_collider_momentums[*collider_idx],
                            attribute,
                        )
                    }
                }
            }
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
