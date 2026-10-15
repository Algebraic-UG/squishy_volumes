// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

mod alembic_mapping;
mod collider;
mod errors;
mod grid_nodes;
mod io_state;
mod particles;
mod read;
mod stats;
mod util;
mod write;

use std::{
    io::{Read, Seek, Write},
    range::Range,
};

use alembic_mapping::*;
use squishy_volumes_util::SplitParameters;
use util::*;

pub use collider::*;
pub use errors::*;
pub use grid_nodes::*;
pub use io_state::*;
pub use particles::*;
pub use stats::*;

use alembic_rust::{ArchiveReader, AttributeElement, Points};

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Frame {
    pub io_state: IoState,
    pub stats: Stats,
    pub error_message: Option<String>,
}
