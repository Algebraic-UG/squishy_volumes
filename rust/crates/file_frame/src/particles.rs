// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use squishy_volumes_util::attribute;
use std::collections::{BTreeMap, BTreeSet};

use crate::{AttributeError, AttributeErrorInner, ParticleObjectError};

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

pub trait Value: Sized {
    fn view(attr: &AttributeValue) -> Result<&[Self], AttributeErrorInner>;
    fn unpack(attr: AttributeValue) -> Result<Vec<Self>, AttributeErrorInner>;
    fn pack(values: Vec<Self>) -> AttributeValue;
}

impl Value for f32 {
    fn view(attr: &AttributeValue) -> Result<&[Self], AttributeErrorInner> {
        if let AttributeValue::Float1(values) = &attr {
            Ok(&values)
        } else {
            Err(AttributeErrorInner::TypeMismatch)
        }
    }

    fn unpack(attr: AttributeValue) -> Result<Vec<Self>, AttributeErrorInner> {
        if let AttributeValue::Float1(values) = attr {
            Ok(values)
        } else {
            Err(AttributeErrorInner::TypeMismatch)
        }
    }
    fn pack(values: Vec<Self>) -> AttributeValue {
        AttributeValue::Float1(values)
    }
}

impl Value for [f32; 2] {
    fn view(attr: &AttributeValue) -> Result<&[Self], AttributeErrorInner> {
        if let AttributeValue::Float2(values) = &attr {
            Ok(&values)
        } else {
            Err(AttributeErrorInner::TypeMismatch)
        }
    }

    fn unpack(attr: AttributeValue) -> Result<Vec<Self>, AttributeErrorInner> {
        if let AttributeValue::Float2(values) = attr {
            Ok(values)
        } else {
            Err(AttributeErrorInner::TypeMismatch)
        }
    }
    fn pack(values: Vec<Self>) -> AttributeValue {
        AttributeValue::Float2(values)
    }
}

impl Value for [f32; 3] {
    fn view(attr: &AttributeValue) -> Result<&[Self], AttributeErrorInner> {
        if let AttributeValue::Float3(values) = &attr {
            Ok(&values)
        } else {
            Err(AttributeErrorInner::TypeMismatch)
        }
    }

    fn unpack(attr: AttributeValue) -> Result<Vec<Self>, AttributeErrorInner> {
        if let AttributeValue::Float3(values) = attr {
            Ok(values)
        } else {
            Err(AttributeErrorInner::TypeMismatch)
        }
    }
    fn pack(values: Vec<Self>) -> AttributeValue {
        AttributeValue::Float3(values)
    }
}

impl AttributeValue {
    fn unpack<T: Value>(self) -> Result<Vec<T>, AttributeErrorInner> {
        T::unpack(self)
    }
    fn pack<T: Value>(values: Vec<T>) -> Self {
        T::pack(values)
    }
}

impl ParticlesObject {
    pub fn view_attr<A>(&self) -> Result<&[A::T], ParticleObjectError>
    where
        A: attribute::Trait,
        A::T: Value,
    {
        self.attributes
            .get(&A::KEY)
            .ok_or(AttributeErrorInner::Missing)
            .and_then(A::T::view)
            .map_err(AttributeError::View)
            .map_err(|error| ParticleObjectError::AttributeError { key: A::KEY, error })
    }

    pub fn replace_attr<A>(
        &mut self,
        values: impl Iterator<Item = A::T>,
    ) -> Result<(), ParticleObjectError>
    where
        A: attribute::Trait,
        A::T: Value,
    {
        *self
            .attributes
            .get_mut(&A::KEY)
            .ok_or(AttributeErrorInner::Missing)
            .map_err(AttributeError::Insert)
            .map_err(|error| ParticleObjectError::AttributeError { key: A::KEY, error })? =
            AttributeValue::pack(values.collect());
        Ok(())
    }

    pub fn remove_attr<A>(&mut self) -> Result<Vec<A::T>, ParticleObjectError>
    where
        A: attribute::Trait,
        A::T: Value,
    {
        self.attributes
            .remove(&A::KEY)
            .ok_or(AttributeErrorInner::Missing)
            .and_then(AttributeValue::unpack)
            .map_err(AttributeError::Remove)
            .map_err(|error| ParticleObjectError::AttributeError { key: A::KEY, error })
    }

    pub fn insert_attr<A>(
        &mut self,
        values: impl Iterator<Item = A::T>,
    ) -> Result<(), ParticleObjectError>
    where
        A: attribute::Trait,
        A::T: Value,
    {
        self.attributes
            .insert(A::KEY, AttributeValue::pack(values.collect()))
            .is_none()
            .ok_or(AttributeErrorInner::Duplicate)
            .map_err(AttributeError::Insert)
            .map_err(|error| ParticleObjectError::AttributeError { key: A::KEY, error })
    }
}
