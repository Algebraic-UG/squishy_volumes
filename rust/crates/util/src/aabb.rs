// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    iter,
    ops::{Add, Sub},
};

use nalgebra::{Vector2, Vector3};

pub trait AabbVector: Add<Output = Self> + Sub<Output = Self> + Sized + Clone + Copy {
    fn splat(value: f32) -> Self;

    fn min(&self, other: &Self) -> Self;
    fn max(&self, other: &Self) -> Self;

    fn leq(&self, other: &Self) -> bool;

    fn lattice(min: Self, extents: Self, spacing: f32) -> (usize, impl Iterator<Item = Self>);
}

impl AabbVector for Vector2<f32> {
    fn splat(value: f32) -> Self {
        Self::repeat(value)
    }

    fn min(&self, other: &Self) -> Self {
        self.inf(other)
    }

    fn max(&self, other: &Self) -> Self {
        self.sup(other)
    }

    fn leq(&self, other: &Self) -> bool {
        self[0] <= other[0] && self[1] <= other[1]
    }

    fn lattice(min: Self, extents: Self, spacing: f32) -> (usize, impl Iterator<Item = Self>) {
        let n = (extents / spacing).map(|x| x.max(1.) as usize);
        (
            n.product(),
            (0..=n.x).flat_map(move |i| {
                (0..=n.y).map(move |j| {
                    min + extents
                        .component_mul(&Self::new(i as f32 / n.x as f32, j as f32 / n.y as f32))
                })
            }),
        )
    }
}

impl AabbVector for Vector3<f32> {
    fn splat(value: f32) -> Self {
        Self::repeat(value)
    }

    fn min(&self, other: &Self) -> Self {
        self.inf(other)
    }

    fn max(&self, other: &Self) -> Self {
        self.sup(other)
    }

    fn leq(&self, other: &Self) -> bool {
        self[0] <= other[0] && self[1] <= other[1] && self[2] <= other[2]
    }

    fn lattice(min: Self, extents: Self, spacing: f32) -> (usize, impl Iterator<Item = Self>) {
        let n = (extents / spacing).map(|x| x.max(1.) as usize);
        (
            n.product(),
            (0..=n.x).flat_map(move |i| {
                (0..=n.y).flat_map(move |j| {
                    (0..=n.z).map(move |k| {
                        min + extents.component_mul(&Self::new(
                            i as f32 / n.x as f32,
                            j as f32 / n.y as f32,
                            k as f32 / n.z as f32,
                        ))
                    })
                })
            }),
        )
    }
}

impl AabbVector for Vector3<i32> {
    fn splat(value: f32) -> Self {
        Self::repeat(value as i32)
    }

    fn min(&self, other: &Self) -> Self {
        self.inf(other)
    }

    fn max(&self, other: &Self) -> Self {
        self.sup(other)
    }

    fn leq(&self, other: &Self) -> bool {
        self[0] <= other[0] && self[1] <= other[1] && self[2] <= other[2]
    }

    fn lattice(_min: Self, _extents: Self, _spacing: f32) -> (usize, impl Iterator<Item = Self>) {
        tracing::error!("lattice for integers not implemented");
        (0, iter::empty())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb<V: AabbVector> {
    pub min: V,
    pub max: V,
}

impl<V: AabbVector> Default for Aabb<V> {
    fn default() -> Self {
        Self {
            min: V::splat(f32::MAX),
            max: V::splat(f32::MIN),
        }
    }
}

impl<V: AabbVector> Aabb<V> {
    pub fn new(points: impl Iterator<Item = V>) -> Self {
        points.fold(Default::default(), Self::extend)
    }

    pub fn extend(self, point: V) -> Self {
        Self {
            min: self.min.min(&point),
            max: self.max.max(&point),
        }
    }

    pub fn extents(&self) -> V {
        self.max - self.min
    }

    pub fn lattice(&self, spacing: f32) -> (usize, impl Iterator<Item = V> + use<V>) {
        V::lattice(self.min, self.extents(), spacing)
    }

    pub fn has_overlap(&self, other: &Self) -> bool {
        self.min.leq(&other.max) && other.min.leq(&self.max)
    }
}
