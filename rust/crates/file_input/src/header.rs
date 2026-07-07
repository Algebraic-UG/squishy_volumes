// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct InputConsts {
    grid_node_size: f32,
    pub leaf_size: f32,
    pub leaf_threshold: u32,
    pub max_num_particles: u32,
    pub simulation_scale: f32,
    pub frames_per_second: u32,
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
}

#[cfg(test)]
impl InputConsts {
    pub fn test_input() -> Self {
        Self {
            leaf_size: 1.,
            leaf_threshold: 16,
            max_num_particles: 10000000,
            grid_node_size: 0.5,
            simulation_scale: 1.,
            frames_per_second: 24,
            domain_min: [-100.; 3],
            domain_max: [100.; 3],
        }
    }
}

impl InputConsts {
    pub fn scaled_grid_node_size(&self) -> f32 {
        self.grid_node_size / self.simulation_scale
    }

    pub fn unscaled_grid_node_size(&self) -> f32 {
        self.grid_node_size
    }

    pub fn accept_distance(&self) -> f32 {
        self.scaled_grid_node_size() * 2.
    }

    pub fn forget_distance(&self) -> f32 {
        self.scaled_grid_node_size() * 2.2
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum InputObject {
    Particles,
    Collider { num_vertices: usize },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct InputHeader {
    pub consts: InputConsts,
    pub objects: std::collections::BTreeMap<String, InputObject>,
}
