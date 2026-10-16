// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::{BTreeMap, BTreeSet};

use squishy_volumes_util::attribute;

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
    fn unpack(attr: AttributeValue) -> Vec<Self>;
    fn pack(values: Vec<Self>) -> AttributeValue;
}

impl Value for f32 {
    fn unpack(attr: AttributeValue) -> Vec<Self> {
        let AttributeValue::Float1(values) = attr else {
            panic!();
        };
        values
    }
    fn pack(values: Vec<Self>) -> AttributeValue {
        AttributeValue::Float1(values)
    }
}

impl Value for [f32; 2] {
    fn unpack(attr: AttributeValue) -> Vec<Self> {
        let AttributeValue::Float2(values) = attr else {
            panic!();
        };
        values
    }
    fn pack(values: Vec<Self>) -> AttributeValue {
        AttributeValue::Float2(values)
    }
}

impl Value for [f32; 3] {
    fn unpack(attr: AttributeValue) -> Vec<Self> {
        let AttributeValue::Float3(values) = attr else {
            panic!();
        };
        values
    }
    fn pack(values: Vec<Self>) -> AttributeValue {
        AttributeValue::Float3(values)
    }
}

impl AttributeValue {
    fn unpack<T: Value>(self) -> Vec<T> {
        T::unpack(self)
    }
    fn pack<T: Value>(values: Vec<T>) -> Self {
        T::pack(values)
    }
}

impl ParticlesObject {
    pub fn remove_attr<A>(&mut self) -> Vec<A::T>
    where
        A: attribute::Trait,
        A::T: Value,
    {
        self.attributes.remove(&A::KEY).unwrap().unpack()
    }

    pub fn insert_attr<A>(&mut self, values: impl Iterator<Item = A::T>)
    where
        A: attribute::Trait,
        A::T: Value,
    {
        if self
            .attributes
            .insert(A::KEY, AttributeValue::pack(values.collect()))
            .is_some()
        {
            panic!();
        }
    }
}
