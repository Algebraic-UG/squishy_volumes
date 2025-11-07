// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::BTreeMap;

use nalgebra::{Quaternion, Vector3};
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;

use crate::setup::Mesh;

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub global: SettingsGlobal,
    pub objects: BTreeMap<String, SettingsObject>,
    pub bulk: BTreeMap<String, BulkData>,
}

#[derive(Serialize, Deserialize)]
pub enum BulkData {
    F32(Vec<f32>),
    I32(Vec<f32>),
}

#[derive(Serialize, Deserialize)]
pub struct SettingsGlobal {
    pub grid_node_size: T,
    pub particle_size: T,
    pub frames_per_second: u32,
    pub gravity: Vector3<T>,
    pub domain_min: Vector3<T>,
    pub domain_max: Vector3<T>,
}

#[derive(Serialize, Deserialize)]
pub struct SettingsObject {
    // to get the right normals on vertices and edges,
    // scaling must be applied before they are computed from the face normals
    pub scale: Vector3<T>,

    pub position: Vector3<T>,
    pub orientation: Quaternion<T>,

    pub linear_velocity: Vector3<T>,
    pub angular_velocity: Vector3<T>,

    pub mesh: SettingsBulk<Mesh>,
    pub ty: SettingsObjectType,
}

#[derive(Serialize, Deserialize)]
pub enum SettingsBulk<T> {
    Unchanged,
    InBulk,
    Loaded(T),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum SettingsObjectType {
    Volume(SettingsObjectVolume),
    Collider(SettingsObjectCollider),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SettingsObjectCollider {
    pub sticky_factor: T,
    pub friction_factor: T,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SettingsObjectVolume {
    pub density: T,
    pub dialation: T,
    pub randomness: T,
    pub viscosity: Option<SettingsViscosity>,
    pub ty: SettingsObjectVolumeType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SettingsViscosity {
    pub dynamic: T,
    pub bulk: T,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum SettingsObjectVolumeType {
    Solid(SettingsObjectVolumeSolid),
    Fluid(SettingsObjectVolumeFluid),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SettingsObjectVolumeSolid {
    pub youngs_modulus: T,
    pub poissons_ratio: T,
    pub sand_alpha: Option<T>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SettingsObjectVolumeFluid {
    pub exponent: i32,
    pub bulk_modulus: T,
}
