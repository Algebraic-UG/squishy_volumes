// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use squishy_volumes_file_frame::{IoState, IoStateError, ParticleObjectError, ParticlesObject};
use squishy_volumes_file_input::{FrameVerifcationError, Header, Object};
use squishy_volumes_util::attribute;
use std::iter::empty;
use thiserror::Error;

use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

#[derive(Error, Debug)]
pub enum AttributeError {
    #[error("Io state error")]
    IoState(#[from] IoStateError),
    #[error("Object is not available {0}")]
    ObjectNotAvailable(String),
    #[error("Frame failed to verify")]
    FrameVerifcationError(#[from] FrameVerifcationError),
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
        attribute: attribute::Key,
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

pub fn available_attributes(header: &Header) -> impl Iterator<Item = Attribute> + '_ {
    empty()
        .chain(AttributeConst::iter().map(Attribute::Const))
        .chain(AttributeGrid::iter().map(Attribute::Grid))
        .chain(
            header
                .objects
                .iter()
                .filter_map(|(name, object)| matches!(object, Object::Particles(_)).then_some(name))
                .flat_map(|name| {
                    attribute::Key::iter().map(|attribute| Attribute::Object {
                        name: name.clone(),
                        attribute,
                    })
                }),
        )
}

pub fn fetch_flat_attribute_f32(
    header: &Header,
    io_state: &IoState,
    attribute: &Attribute,
) -> Result<Vec<f32>, AttributeError> {
    let scale = header.consts.simulation_scale;

    Ok(match attribute {
        Attribute::Const(attribute) => match attribute {
            AttributeConst::GridNodeSize => vec![header.consts.scaled_grid_node_size()],
            AttributeConst::SimulationScale => vec![header.consts.simulation_scale],
            AttributeConst::DomainMin => header.consts.unscaled_domain_min().to_vec(),
            AttributeConst::DomainMax => header.consts.unscaled_domain_max().to_vec(),
            _ => Err(AttributeError::NotFloatAttribute(format!("{attribute:?}")))?,
        },
        Attribute::Object { name, attribute } => {
            struct Visitor<'a>(&'a ParticlesObject);
            impl<'a> attribute::Visitor for Visitor<'a> {
                type Output = Result<&'a [f32], ParticleObjectError>;

                fn visit<A: attribute::Trait>(&mut self) -> Self::Output {
                    Ok(bytemuck::cast_slice(self.0.view_attr::<A>()?))
                }
            }
            let mut attr = attribute
                .visit(&mut Visitor(io_state.get_particle_object(name)?))
                .map_err(|error| IoStateError::ParticleObject {
                    name: name.clone(),
                    error,
                })?
                .to_vec();
            if attribute.scale() {
                attr.iter_mut().for_each(|f| *f *= scale);
            }
            attr
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
                            .iter()
                            .map(|c| *c as f32 * header.consts.unscaled_grid_node_size())
                    })
                    .collect(),
                AttributeGrid::Velocities => bytemuck::cast_slice(&grid_nodes.velocites).to_vec(),
                _ => Err(AttributeError::NotFloatAttribute(format!("{attribute:?}")))?,
            }
        }
    })
}

pub fn fetch_flat_attribute_i32(
    header: &Header,
    io_state: &IoState,
    attribute: &Attribute,
) -> Result<Vec<i32>, AttributeError> {
    Ok(match attribute {
        Attribute::Const(attribute) => match attribute {
            AttributeConst::FramesPerSecond => vec![header.consts.frames_per_second as i32],
            _ => Err(AttributeError::NotIntAttribute(format!("{attribute:?}")))?,
        },
        Attribute::Object { .. } => {
            unimplemented!("currently no i32 attributes")
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
