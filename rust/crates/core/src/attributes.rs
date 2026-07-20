// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use squishy_volumes_file_frame::IoState;
use squishy_volumes_file_input::{InputHeader, InputRange, InputRanges, ObjectError};
use std::iter::empty;
use thiserror::Error;

use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

#[derive(Error, Debug)]
pub enum AttributeError {
    #[error("Object error: {0}")]
    ObjectError(#[from] ObjectError),
    #[error("This is not a float attribute: {0}")]
    NotFloatAttribute(String),
    #[error("This is not a int attribute: {0}")]
    NotIntAttribute(String),
    #[error("The grid was not stored")]
    NoGridStored,
}

#[derive(Serialize, Deserialize)]
pub enum Attribute {
    Const(AttributeConst),
    Object {
        name: String,
        attribute: AttributeParticles,
    },
    Grid(AttributeGrid),
}

#[derive(Debug, Default, EnumIter, Serialize, Deserialize)]
pub enum AttributeConst {
    #[default]
    GridNodeSize,
    FramesPerSecond,
    SimulationScale,
    DomainMin,
    DomainMax,
}

#[derive(Debug, EnumIter, Serialize, Deserialize)]
pub enum AttributeGrid {
    Masses,
    Positions,
    Velocities,
    ColliderBits,
}

#[derive(Debug, EnumIter, Serialize, Deserialize)]
pub enum AttributeParticles {
    Flags,
    Masses,
    InitialVolumes,
    Positions,
    InitialPositions,
    Velocities,
    PositionGradients,
    ElasticEnergies,
    Sizes,
    Transformations,
    ColliderBits,
}

pub fn available_attributes(input_header: &InputHeader) -> impl Iterator<Item = Attribute> + '_ {
    empty()
        .chain(AttributeConst::iter().map(Attribute::Const))
        .chain(AttributeGrid::iter().map(Attribute::Grid))
        .chain(input_header.objects.keys().flat_map(|name| {
            AttributeParticles::iter().map(|attribute| Attribute::Object {
                name: name.clone(),
                attribute,
            })
        }))
}

pub fn fetch_flat_attribute_f32(
    input_header: &InputHeader,
    input_ranges: &InputRanges,
    io_state: &IoState,
    attribute: &Attribute,
) -> Result<Vec<f32>, AttributeError> {
    let scale = input_header.consts.simulation_scale;
    let inv_scale = 1. / scale;

    Ok(match attribute {
        Attribute::Const(attribute) => match attribute {
            AttributeConst::GridNodeSize => vec![input_header.consts.scaled_grid_node_size()],
            AttributeConst::SimulationScale => vec![input_header.consts.simulation_scale],
            AttributeConst::DomainMin => input_header.consts.domain_min.to_vec(),
            AttributeConst::DomainMax => input_header.consts.domain_max.to_vec(),
            _ => Err(AttributeError::NotFloatAttribute(format!("{attribute:?}")))?,
        },
        Attribute::Object { name, attribute } => {
            let particle_range = input_ranges.get_particle_range(name)?;

            match attribute {
                AttributeParticles::Masses => io_state.particles.parameters.as_slice()
                    [particle_range]
                    .iter()
                    .map(|parameters| parameters.mass)
                    .collect(),
                AttributeParticles::InitialVolumes => io_state.particles.parameters.as_slice()
                    [particle_range]
                    .iter()
                    .map(|parameters| parameters.initial_volume * scale.powi(3))
                    .collect(),
                AttributeParticles::Positions => bytemuck::cast_slice::<_, f32>(
                    &io_state.particles.positions.as_slice()[particle_range],
                )
                .iter()
                .map(|&p| p * scale)
                .collect(),
                AttributeParticles::InitialPositions => bytemuck::cast_slice(
                    &io_state.particles.initial_positions.as_slice()[particle_range],
                )
                .to_vec(),
                AttributeParticles::Velocities => {
                    bytemuck::cast_slice(&io_state.particles.velocities.as_slice()[particle_range])
                        .to_vec()
                }
                AttributeParticles::PositionGradients => bytemuck::cast_slice(
                    &io_state.particles.position_gradients.as_slice()[particle_range],
                )
                .to_vec(),
                AttributeParticles::ElasticEnergies => {
                    io_state.particles.elastic_energies.as_slice()[particle_range].to_vec()
                }
                AttributeParticles::Sizes => io_state.particles.parameters.as_slice()
                    [particle_range]
                    .iter()
                    .map(|parameters| parameters.initial_volume.powf(1. / 3.) * scale)
                    .collect(),
                AttributeParticles::Transformations => io_state.particles.positions.as_slice()
                    [particle_range.clone()]
                .iter()
                .zip(io_state.particles.position_gradients.as_slice()[particle_range].iter())
                .flat_map(|(position, position_gradient)| {
                    let [[m00, m01, m02], [m10, m11, m12], [m20, m21, m22]] = *position_gradient;
                    let [m30, m31, m32] = *position;
                    [
                        m00,
                        m01,
                        m02,
                        0.,
                        m10,
                        m11,
                        m12,
                        0.,
                        m20,
                        m21,
                        m22,
                        0.,
                        scale * m30,
                        scale * m31,
                        scale * m32,
                        1.,
                    ]
                })
                .collect(),
                _ => Err(AttributeError::NotFloatAttribute(format!("{attribute:?}")))?,
            }
        }
        Attribute::Grid(attribute) => {
            let grid_nodes = io_state
                .grid_nodes
                .as_ref()
                .ok_or(AttributeError::NoGridStored)?;
            match attribute {
                AttributeGrid::Masses => grid_nodes.masses.clone(),
                AttributeGrid::Positions => grid_nodes
                    .node_ids
                    .iter()
                    .flat_map(|node_id| {
                        node_id
                            .into_iter()
                            .map(|c| *c as f32 * input_header.consts.unscaled_grid_node_size())
                    })
                    .collect(),
                AttributeGrid::Velocities => bytemuck::cast_slice(&grid_nodes.velocites).to_vec(),
                _ => Err(AttributeError::NotFloatAttribute(format!("{attribute:?}")))?,
            }
        }
    })
}

pub fn fetch_flat_attribute_i32(
    input_header: &InputHeader,
    input_ranges: &InputRanges,
    io_state: &IoState,
    attribute: &Attribute,
) -> Result<Vec<i32>, AttributeError> {
    Ok(match attribute {
        Attribute::Const(attribute) => match attribute {
            AttributeConst::FramesPerSecond => vec![input_header.consts.frames_per_second as i32],
            _ => Err(AttributeError::NotIntAttribute(format!("{attribute:?}")))?,
        },
        Attribute::Object { name, attribute } => {
            let particle_range = input_ranges.get_particle_range(name)?;

            match attribute {
                AttributeParticles::Flags => {
                    bytemuck::cast_slice(&io_state.particles.flags.as_slice()[particle_range])
                        .to_vec()
                }
                AttributeParticles::ColliderBits => bytemuck::cast_slice(
                    &io_state.particles.collider_bits.as_slice()[particle_range],
                )
                .to_vec(),
                _ => Err(AttributeError::NotIntAttribute(format!("{attribute:?}")))?,
            }
        }
        Attribute::Grid(attribute) => {
            let grid_nodes = io_state
                .grid_nodes
                .as_ref()
                .ok_or(AttributeError::NoGridStored)?;
            match attribute {
                AttributeGrid::ColliderBits => {
                    bytemuck::cast_slice(&grid_nodes.collider_bits).to_vec()
                }
                _ => Err(AttributeError::NotIntAttribute(format!("{attribute:?}")))?,
            }
        }
    })
}
