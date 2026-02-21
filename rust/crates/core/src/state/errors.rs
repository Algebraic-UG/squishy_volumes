// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AttributeError {
    #[error("Failed to deserialize key: {0}")]
    KeyDeserialization(#[from] serde_json::Error),
}
