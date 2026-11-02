// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
