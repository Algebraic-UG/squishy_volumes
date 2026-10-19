// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::BTreeMap;

use squishy_volumes_util::{AnimatedGlobals, attribute};
use thiserror::Error;

use super::*;

#[derive(Error, Debug)]
pub enum IoStateError {
    #[error("Particle object error '{name}'")]
    ParticleObject {
        name: String,
        #[source]
        error: ParticleObjectError,
    },
}

#[derive(Error, Debug)]
pub enum ParticleObjectError {
    #[error("The IoState doesn't contain this object.")]
    Missing,
    #[error("An attribute error occurred '{key:?}'")]
    AttributeError {
        key: attribute::Key,
        #[source]
        error: AttributeError,
    },
}

#[derive(Error, Debug)]
pub enum AttributeError {
    #[error("Inserting")]
    Insert(#[source] AttributeErrorInner),
    #[error("Removing")]
    Remove(#[source] AttributeErrorInner),
    #[error("Replacing")]
    Replace(#[source] AttributeErrorInner),
    #[error("View")]
    View(#[source] AttributeErrorInner),
}

#[derive(Error, Debug)]
pub enum AttributeErrorInner {
    #[error("Attribute missing")]
    Missing,
    #[error("Attribute duplication")]
    Duplicate,
    #[error("Attribute type mismatch")]
    TypeMismatch,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct IoState {
    pub time: f64,

    pub animated_globals: AnimatedGlobals,

    pub particle_objects: BTreeMap<String, ParticlesObject>,

    pub collider: Collider,

    // these can be computed from scratch, so they are optional
    pub bvh: Option<squishy_volumes_mesh_util::BoundingVolumeHierarchy>,
    pub grid_nodes: Option<GridNodes>,
}

impl IoState {
    pub fn get_particle_object(&self, name: &str) -> Result<&ParticlesObject, IoStateError> {
        self.particle_objects
            .get(name)
            .ok_or(IoStateError::ParticleObject {
                name: name.to_string(),
                error: ParticleObjectError::Missing,
            })
    }
    pub fn get_particle_object_mut(
        &mut self,
        name: &str,
    ) -> Result<&mut ParticlesObject, IoStateError> {
        self.particle_objects
            .get_mut(name)
            .ok_or(IoStateError::ParticleObject {
                name: name.to_string(),
                error: ParticleObjectError::Missing,
            })
    }

    pub fn collect<A: attribute::Trait>(
        &mut self,
    ) -> Result<impl Iterator<Item = A::T> + use<A>, IoStateError>
    where
        A::T: Value,
    {
        Ok(self
            .particle_objects
            .iter_mut()
            .map(|(name, particle_object)| {
                particle_object
                    .remove_attr::<A>()
                    .map_err(|error| IoStateError::ParticleObject {
                        name: name.clone(),
                        error,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten())
    }
}
