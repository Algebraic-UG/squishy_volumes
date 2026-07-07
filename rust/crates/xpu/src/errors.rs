// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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

    #[error("Something went wrong with the compute harness: {0}")]
    HarnessError(#[from] super::HarnessError),
}
