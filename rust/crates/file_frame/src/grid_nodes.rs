// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GridNodes {
    pub node_ids: Vec<[i32; 3]>,
    pub collider_bits: Vec<u32>,
    pub masses: Vec<f32>,
    pub velocites: Vec<[f32; 3]>,
}
