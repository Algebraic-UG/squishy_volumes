// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Collider {
    pub vertex_positions: Vec<[f32; 3]>,
    pub triangle_frictions: Vec<f32>,
    pub triangle_dampings: Vec<f32>,
}

impl Collider {
    /// Multiplies every vertex position by `factor`.
    pub fn scale_lengths(&mut self, factor: f32) {
        self.vertex_positions
            .iter_mut()
            .flatten()
            .for_each(|v| *v *= factor);
    }
}
