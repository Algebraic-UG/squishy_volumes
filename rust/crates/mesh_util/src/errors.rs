// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("'{name}': non-manifold edge between {vertex_index_a} and {vertex_index_b}")]
    NonManifoldEdge {
        name: String,
        vertex_index_a: u32,
        vertex_index_b: u32,
    },

    #[error("'{name}': vertex index out of range in triangle {triangle_index}")]
    VertexIndexOutOfRange { name: String, triangle_index: usize },
}
