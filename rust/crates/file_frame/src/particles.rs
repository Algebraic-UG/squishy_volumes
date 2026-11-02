// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use squishy_volumes_util::attribute;
use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet},
    mem::take,
};

use crate::{AttributeError, ParticleObjectError};

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParticlesObject {
    pub num_particles: usize,

    pub alembic_attributes: BTreeSet<attribute::Key>,
    pub attributes: BTreeMap<attribute::Key, AttributeValue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AttributeValue {
    Float1(Vec<f32>),
    Float2(Vec<[f32; 2]>),
    Float3(Vec<[f32; 3]>),
}

pub trait Value: Sized + Default {
    fn as_slice_mut(attr: &mut AttributeValue) -> Result<&mut [Self], AttributeError>;
    fn as_slice(attr: &AttributeValue) -> Result<&[Self], AttributeError>;
    fn unpack(attr: AttributeValue) -> Result<Vec<Self>, AttributeError>;
    fn pack(values: Vec<Self>) -> AttributeValue;
}

impl<T: alembic_rust::AttributeElement> Value for T {
    fn as_slice_mut(attr: &mut AttributeValue) -> Result<&mut [Self], AttributeError> {
        match attr {
            AttributeValue::Float1(items) => items as &mut dyn Any,
            AttributeValue::Float2(items) => items as &mut dyn Any,
            AttributeValue::Float3(items) => items as &mut dyn Any,
        }
        .downcast_mut::<Vec<Self>>()
        .ok_or(AttributeError::TypeMismatch)
        .map(Vec::as_mut_slice)
    }

    fn as_slice(attr: &AttributeValue) -> Result<&[Self], AttributeError> {
        match attr {
            AttributeValue::Float1(items) => items as &dyn Any,
            AttributeValue::Float2(items) => items as &dyn Any,
            AttributeValue::Float3(items) => items as &dyn Any,
        }
        .downcast_ref::<Vec<Self>>()
        .ok_or(AttributeError::TypeMismatch)
        .map(Vec::as_slice)
    }

    fn unpack(mut attr: AttributeValue) -> Result<Vec<Self>, AttributeError> {
        match &mut attr {
            AttributeValue::Float1(items) => items as &mut dyn Any,
            AttributeValue::Float2(items) => items as &mut dyn Any,
            AttributeValue::Float3(items) => items as &mut dyn Any,
        }
        .downcast_mut::<Vec<Self>>()
        .ok_or(AttributeError::TypeMismatch)
        .map(take)
    }

    fn pack(mut values: Vec<Self>) -> AttributeValue {
        let values = &mut values as &mut dyn Any;
        if let Some(values) = values.downcast_mut() {
            return AttributeValue::Float1(take(values));
        }
        if let Some(values) = values.downcast_mut() {
            return AttributeValue::Float2(take(values));
        }
        if let Some(values) = values.downcast_mut() {
            return AttributeValue::Float3(take(values));
        }
        unimplemented!("There's a new value type that can't be packed yet");
    }
}

impl AttributeValue {
    fn scale(&mut self, factor: f32) {
        match self {
            AttributeValue::Float1(items) => items.iter_mut().for_each(|v| *v *= factor),
            AttributeValue::Float2(items) => items.iter_mut().flatten().for_each(|v| *v *= factor),
            AttributeValue::Float3(items) => items.iter_mut().flatten().for_each(|v| *v *= factor),
        }
    }

    fn unpack<T: Value>(self) -> Result<Vec<T>, AttributeError> {
        T::unpack(self)
    }
    fn pack<T: Value>(values: Vec<T>) -> Self {
        T::pack(values)
    }
}

impl ParticlesObject {
    /// Multiplies every attribute that carries a length by `factor`.
    pub fn scale_lengths(&mut self, factor: f32) {
        self.attributes
            .iter_mut()
            .filter(|(key, _)| key.scale())
            .for_each(|(_, value)| value.scale(factor));
    }

    pub fn view_attr_mut<A: attribute::Trait>(
        &mut self,
    ) -> Result<&mut [A::T], ParticleObjectError> {
        self.attributes
            .get_mut(&A::KEY)
            .ok_or(AttributeError::Missing)
            .and_then(A::T::as_slice_mut)
            .map_err(|error| ParticleObjectError::AttributeError { key: A::KEY, error })
    }

    pub fn view_attr<A: attribute::Trait>(&self) -> Result<&[A::T], ParticleObjectError> {
        self.attributes
            .get(&A::KEY)
            .ok_or(AttributeError::Missing)
            .and_then(A::T::as_slice)
            .map_err(|error| ParticleObjectError::AttributeError { key: A::KEY, error })
    }

    pub fn replace_attr<A: attribute::Trait>(
        &mut self,
        values: impl Iterator<Item = A::T>,
    ) -> Result<(), ParticleObjectError> {
        *self
            .attributes
            .get_mut(&A::KEY)
            .ok_or(AttributeError::Missing)
            .map_err(|error| ParticleObjectError::AttributeError { key: A::KEY, error })? =
            AttributeValue::pack(values.collect());
        Ok(())
    }

    pub fn remove_attr<A: attribute::Trait>(&mut self) -> Result<Vec<A::T>, ParticleObjectError> {
        self.attributes
            .remove(&A::KEY)
            .ok_or(AttributeError::Missing)
            .and_then(AttributeValue::unpack)
            .map_err(|error| ParticleObjectError::AttributeError { key: A::KEY, error })
    }

    pub fn insert_attr<A: attribute::Trait>(
        &mut self,
        values: impl Iterator<Item = A::T>,
    ) -> Result<(), ParticleObjectError> {
        self.attributes
            .insert(A::KEY, AttributeValue::pack(values.collect()))
            .is_none()
            .ok_or(AttributeError::Duplicate)
            .map_err(|error| ParticleObjectError::AttributeError { key: A::KEY, error })
    }
}
