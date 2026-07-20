// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ColliderInputs(std::collections::BTreeMap<String, ColliderInput>);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub struct ColliderInput {
    pub vertex_positions: Vec<[f32; 3]>,
    pub triangle_indices: Vec<[u32; 3]>,
    pub triangle_frictions: Vec<f32>,
}

impl std::ops::Deref for ColliderInputs {
    type Target = std::collections::BTreeMap<String, ColliderInput>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
impl ColliderInput {
    fn random(num_vertices: usize, num_triangles: usize, rng: &mut impl rand::Rng) -> Self {
        use rand::RngExt as _;
        Self {
            vertex_positions: rng.random_iter().take(num_vertices).collect(),
            triangle_indices: rng.random_iter().take(num_triangles).collect(),
            triangle_frictions: rng.random_iter().take(num_triangles).collect(),
        }
    }
}

impl ColliderInputs {
    pub fn into_values(self) -> std::collections::btree_map::IntoValues<String, ColliderInput> {
        self.0.into_values()
    }

    pub fn entry<'a>(
        &'a mut self,
        key: String,
    ) -> Result<std::collections::btree_map::Entry<'a, String, ColliderInput>, crate::InputError>
    {
        let full = self.0.len() == 16;
        let entry = self.0.entry(key);
        if full && matches!(entry, std::collections::btree_map::Entry::Vacant(_)) {
            Err(crate::InputError::TooManyColliders)
        } else {
            Ok(entry)
        }
    }
}
