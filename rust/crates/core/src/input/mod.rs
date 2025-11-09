// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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
mod reading;
mod writing;

#[cfg(test)]
mod tests;

use common::*;

pub use frame::{BulkData, InputFrame};
pub use header::InputHeader;
pub use reading::InputReader;
pub use writing::InputWriter;
