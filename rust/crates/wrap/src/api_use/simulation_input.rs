// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Context, Result};
use numpy::PyReadonlyArray1;
use pyo3::prelude::*;
use serde_json::from_str;
use squishy_volumes_api::InputBulk;

use crate::hot_reloadable::try_with_context;

#[pyfunction]
pub fn start_frame(frame_start: &str) -> Result<()> {
    try_with_context(|context| {
        context
            .get_simulation_input()
            .context("Not recording input")?
            .start_frame(from_str(frame_start).context("Frame start string isn't valid JSON")?)
    })
}

#[pyfunction]
pub fn record_input_float<'py>(meta: &str, bulk: PyReadonlyArray1<'py, f32>) -> Result<()> {
    try_with_context(|context| {
        context
            .get_simulation_input()
            .context("Not recording input")?
            .record_input(
                from_str(meta).context("Meta string isn't valid JSON")?,
                InputBulk::F32(bulk.as_slice()?),
            )
    })
}

#[pyfunction]
pub fn record_input_int<'py>(meta: &str, bulk: PyReadonlyArray1<'py, i32>) -> Result<()> {
    try_with_context(|context| {
        context
            .get_simulation_input()
            .context("Not recording input")?
            .record_input(
                from_str(meta).context("Meta string isn't valid JSON")?,
                InputBulk::I32(bulk.as_slice()?),
            )
    })
}
#[pyfunction]
pub fn finish_frame() -> Result<()> {
    try_with_context(|context| {
        context
            .get_simulation_input()
            .context("Not recording input")?
            .finish_frame()
    })
}
