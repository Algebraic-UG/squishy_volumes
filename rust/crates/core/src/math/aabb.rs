// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::ops::{Add, Mul, Sub};

use nalgebra::{Vector2, Vector3};
use squishy_volumes_api::T;

pub trait AabbVector:
    Add<Output = Self> + Sub<Output = Self> + Mul<T, Output = Self> + Sized + Clone + Copy
{
    fn splat(value: T) -> Self;
    fn min(&self, other: &Self) -> Self;
    fn max(&self, other: &Self) -> Self;

    fn lattice(min: Self, extents: Self, spacing: T) -> (usize, impl Iterator<Item = Self>);
}

impl AabbVector for Vector2<T> {
    fn splat(value: T) -> Self {
        Self::repeat(value)
    }

    fn min(&self, other: &Self) -> Self {
        self.inf(other)
    }

    fn max(&self, other: &Self) -> Self {
        self.sup(other)
    }

    fn lattice(min: Self, extents: Self, spacing: T) -> (usize, impl Iterator<Item = Self>) {
        let n = (extents / spacing).map(|x| x.max(1.) as usize);
        (
            n.product(),
            (0..=n.x).flat_map(move |i| {
                (0..=n.y).map(move |j| {
                    min + extents.component_mul(&Self::new(i as T / n.x as T, j as T / n.y as T))
                })
            }),
        )
    }
}

impl AabbVector for Vector3<T> {
    fn splat(value: T) -> Self {
        Self::repeat(value)
    }

    fn min(&self, other: &Self) -> Self {
        self.inf(other)
    }

    fn max(&self, other: &Self) -> Self {
        self.sup(other)
    }

    fn lattice(min: Self, extents: Self, spacing: T) -> (usize, impl Iterator<Item = Self>) {
        let n = (extents / spacing).map(|x| x.max(1.) as usize);
        (
            n.product(),
            (0..=n.x).flat_map(move |i| {
                (0..=n.y).flat_map(move |j| {
                    (0..=n.z).map(move |k| {
                        min + extents.component_mul(&Self::new(
                            i as T / n.x as T,
                            j as T / n.y as T,
                            k as T / n.z as T,
                        ))
                    })
                })
            }),
        )
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
            min: V::splat(T::MAX),
            max: V::splat(T::MIN),
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

    pub fn lattice(&self, spacing: T) -> (usize, impl Iterator<Item = V>) {
        V::lattice(self.min, self.extents(), spacing)
    }
}
