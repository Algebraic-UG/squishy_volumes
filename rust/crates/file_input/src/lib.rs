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

// The input is a binary file that contains a mix of parameters and bulk geometry.
//
// The file is meant to be filled quickly, so there is minimal processing done on the bulk.
// At some point before the input is used in the simulation, additional processing must happen.
//
// The structure is simple, there are a few things that should remain stable across versions
// followed by a bunch of things that are almost completely handled by serde.
// So those typically break between versions, but there might be migration paths later.
//
// =============================================================================
// Stable:
// =============================================================================
//
// 32 Magic bytes: binary of "Squishy Volumes Input File Magic"
// 64 Version Bytes: any version string like "10.1337.42-alpha" should fit this
//
// =============================================================================
// Unstable:
// =============================================================================
//
// InputHeader: contains everything that is known from the start of input recording
//
// InputFrame: Potentially bulky input from frame 0
// InputFrame: Potentially bulky input from frame 1
// InputFrame: Potentially bulky input from frame 2
// ...
//
// Index: contains all the frame offsets and is constructed in memory while recording
//
// 8 Index length bytes: so one can jump to the start of the index (not handled by serde!)

mod common;
mod frame;
mod header;
#[cfg(test)]
mod python_constants;
mod reading;
mod writing;

#[cfg(test)]
mod tests;

use common::*;

mod errors;
pub use errors::*;

pub use frame::*;
pub use header::*;

pub use reading::InputReader;
pub use writing::InputWriter;
