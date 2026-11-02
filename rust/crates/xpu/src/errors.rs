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

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No frames to interpolate.")]
    NoFrames,

    #[error("'{name}': length mismatch between '{attribute_a}' and '{attribute_b}'")]
    AttributeLengthMismatch {
        name: String,
        attribute_a: String,
        attribute_b: String,
    },

    #[error("'{name}': flattened '{attribute}' is not multiple of {multiple}")]
    FlattedNotCorrectMultiple {
        name: String,
        attribute: String,
        multiple: usize,
    },

    #[error("Something went wrong with the compute harness")]
    HarnessError(#[from] super::HarnessError),
}
