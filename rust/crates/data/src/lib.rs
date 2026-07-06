// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Object {
    Particles(usize),
    Collider(usize),
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ParticlesObject {
    pub indices: Vec<u32>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ColliderObject {
    pub index: u32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct DataState {
    pub time: f64,

    pub objects: std::collections::BTreeMap<String, Object>,

    pub particles: Particles,

    pub grid: Option<GridNodes>,

    pub user_data: Vec<u8>,
}

#[repr(C)]
#[derive(
    Clone,
    Copy,
    bytemuck::Zeroable,
    bytemuck::Pod,
    Debug,
    PartialEq,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ParticleFlags(u32);

bitflags::bitflags! {
    impl ParticleFlags: u32{
        const IS_SOLID = 1 << 0;
        const IS_FLUID = 1 << 1;
        const USE_VISCOSITY = 1 << 2;
        const USE_SAND_ALPHA = 1 << 3;
        const HAS_GOAL = 1 << 4;
        const TOMBSTONED = 1 << 5;
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ViscosityParameters {
    pub dynamic: f32,
    pub bulk: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParticleParameters {
    pub mass: f32,
    pub initial_volume: f32,
    pub viscosity: Option<ViscosityParameters>,
    pub specific: SpecificParticleParameters,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SpecificParticleParameters {
    Solid {
        mu: f32,
        lambda: f32,
        sand_alpha: Option<f32>,
    },
    Fluid {
        exponent: i32,
        bulk_modulus: f32,
    },
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Particles {
    pub flags: Vec<ParticleFlags>,

    pub parameters: Vec<ParticleParameters>,

    pub collider_bits: Vec<u32>,

    pub positions: Vec<[f32; 3]>,
    pub position_gradients: Vec<[f32; 9]>,

    pub velocities: Vec<[f32; 3]>,
    pub velocity_gradients: Vec<[f32; 9]>,

    pub initial_positions: Vec<[f32; 3]>,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GridNodes {
    pub node_ids: Vec<[i32; 3]>,
    pub collider_bits: Vec<u32>,
    pub masses: Vec<f32>,
    pub velocites: Vec<[f32; 3]>,
}
