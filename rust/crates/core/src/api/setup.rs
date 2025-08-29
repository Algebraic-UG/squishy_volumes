// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use blended_mpm_api::T;
use nalgebra::{Quaternion, Vector3};
use serde::{Deserialize, Serialize};

use super::Mesh;

#[derive(Clone, Serialize, Deserialize)]
pub enum ObjectSettings {
    Solid(ObjectSettingsSolid),
    Fluid(ObjectSettingsFluid),
    Collider(ObjectSettingsCollider),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ObjectSettingsSolid {
    pub density: T,
    pub youngs_modulus: T,
    pub poissons_ratio: T,
    pub viscosity: T,
    pub dilation: T,
    pub randomness: T,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ObjectSettingsFluid {
    pub density: T,
    pub exponent: i32,
    pub viscosity: T,
    pub bulk_modulus: T,
    pub dilation: T,
    pub randomness: T,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ObjectSettingsCollider {
    pub sticky_factor: T,
    pub friction_factor: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptedFrame {
    pub position: Vector3<T>,
    pub orientation: Quaternion<T>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Object {
    pub name: String,

    // to get the right normals on vertices and edges, this is applied on deserialization
    pub scale: Vector3<T>,

    pub position: Vector3<T>,
    pub orientation: Quaternion<T>,

    pub linear_velocity: Vector3<T>,
    pub angular_velocity: Vector3<T>,

    pub settings: ObjectSettings,
}

#[derive(Serialize, Deserialize)]
pub struct GlobalSettings {
    pub grid_node_size: T,
    pub particle_size: T,
    pub frames_per_second: u32,
    pub gravity: Vector3<T>,
}

pub struct Setup {
    pub settings: GlobalSettings,
    pub objects: Vec<ObjectWithData>,
}

pub struct ObjectWithData {
    pub object: Object,
    pub mesh: Mesh,
    pub scripted_frames: Vec<ScriptedFrame>,
}
