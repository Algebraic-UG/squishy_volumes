// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Context as _, Result};
use pyo3::prelude::*;
use serde_json::from_str;

use crate::hot_reloadable::{try_with_context, with_context};

#[pyfunction]
pub fn new_simulation_input(
    uuid: String,
    directory: String,
    input_header: &str,
    max_bytes_on_disk: u64,
) -> Result<()> {
    try_with_context(move |context| {
        context.new_simulation_input(
            uuid,
            directory.into(),
            from_str(input_header).context("Input header string isn't valid JSON")?,
            max_bytes_on_disk,
        )
    })
}

#[pyfunction]
pub fn new_simulation() -> Result<()> {
    try_with_context(|context| context.new_simulation())
}

#[pyfunction]
pub fn load_simulation(uuid: String, directory: String) -> Result<()> {
    try_with_context(move |context| context.load_simulation(uuid, directory.into()))
}

#[pyfunction]
pub fn drop_simulation(uuid: &str) -> Result<()> {
    with_context(move |context| context.drop_simulation(uuid))
}
