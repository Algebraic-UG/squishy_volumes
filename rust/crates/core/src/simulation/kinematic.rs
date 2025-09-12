// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Context, Error, Result, ensure};
use nalgebra::{Matrix1, Matrix1x3, Matrix3, Matrix4, UnitQuaternion, Vector3, stack};
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;

use crate::{
    api::Object,
    math::{NORMALIZATION_EPS, SLERP_EPS},
};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Kinematic {
    pub position: Vector3<T>,
    pub orientation: UnitQuaternion<T>,

    pub linear_velocity: Vector3<T>,
    pub angular_velocity: Vector3<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptedMovement {
    pub time: T,
    pub position: Vector3<T>,
    pub orientation: UnitQuaternion<T>,
}

impl ScriptedMovement {
    pub fn find_iterpolation_pair(candidates: &[Self], time: f64) -> Option<(&Self, &Self)> {
        Some((
            candidates
                .iter()
                .rev()
                .find(|movement| movement.time as f64 <= time)?,
            candidates
                .iter()
                .find(|movement| movement.time as f64 >= time)?,
        ))
    }
}

impl Kinematic {
    pub fn interpolate(a: &ScriptedMovement, b: &ScriptedMovement, time: f64) -> Result<Self> {
        if a.time == b.time {
            return Ok(Self {
                position: a.position,
                orientation: a.orientation,
                linear_velocity: Vector3::zeros(),
                angular_velocity: Vector3::zeros(),
            });
        }

        let time_span = b.time - a.time;
        let passed_time = (time - a.time as f64) as T;
        let factor = passed_time / time_span;

        let position = a.position * (1. - factor) + b.position * factor;

        let alignment = a.orientation.coords.dot(&b.orientation.coords);
        let orientation = if alignment.abs() > 0.9 {
            // in good alignment we rather use linear interpolation
            // also making sure to use the 'right' version of the orientation
            a.orientation.nlerp(
                UnitQuaternion::from_ref_unchecked(
                    &(b.orientation.quaternion() * if alignment < 0. { -1. } else { 1. }),
                ),
                factor,
            )
        } else {
            a.orientation
                .try_slerp(&b.orientation, factor, SLERP_EPS)
                .context("Orientation jump too large")?
        };

        let linear_velocity = (b.position - a.position) / time_span;
        let angular_velocity =
            (a.orientation.conjugate() * b.orientation).coords.xyz() * (2. / time_span);

        Ok(Self {
            position,
            orientation,
            linear_velocity,
            angular_velocity,
        })
    }

    #[allow(clippy::toplevel_ref_arg)]
    pub fn transformation(&self) -> Matrix4<T> {
        let matrix_part: Matrix3<T> = self.orientation.to_rotation_matrix().into();
        stack![
            matrix_part, self.position;
            Matrix1x3::zeros(), Matrix1::new(1.);
        ]
    }

    pub fn point_velocity_from_world(&self, world_position: Vector3<T>) -> Vector3<T> {
        self.linear_velocity
            + self
                .orientation
                .transform_vector(&self.angular_velocity)
                .cross(&(world_position - self.position))
    }

    pub fn point_velocity_from_local(&self, local_position: Vector3<T>) -> Vector3<T> {
        self.linear_velocity
            + self
                .orientation
                .transform_vector(&self.angular_velocity.cross(&local_position))
    }

    pub fn to_world_position(&self, local_position: Vector3<T>) -> Vector3<T> {
        self.orientation.transform_vector(&local_position) + self.position
    }

    pub fn to_world_normal(&self, local_normal: Vector3<T>) -> Vector3<T> {
        self.orientation.transform_vector(&local_normal)
    }
}

impl TryFrom<Object> for Kinematic {
    type Error = Error;
    fn try_from(
        Object {
            position,
            orientation,
            linear_velocity,
            angular_velocity,
            ..
        }: Object,
    ) -> Result<Self> {
        ensure!(
            (orientation.coords.norm() - 1.).abs() < NORMALIZATION_EPS,
            "Orientation quaternion isn't normalized"
        );
        Ok(Self {
            position,
            orientation: UnitQuaternion::from_quaternion(orientation),
            linear_velocity,
            angular_velocity,
        })
    }
}
