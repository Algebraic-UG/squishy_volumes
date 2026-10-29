// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

mod collider;
mod errors;
mod grid_nodes;
mod io_state;
mod particles;
mod read;
mod util;
mod write;

use std::io::{Read, Seek, Write};

pub use util::*;

pub use collider::*;
pub use errors::*;
pub use grid_nodes::*;
pub use io_state::*;
pub use particles::*;

use alembic_rust::{ArchiveReader, Points};

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Frame {
    pub io_state: IoState,
    pub error_message: Option<String>,
}
