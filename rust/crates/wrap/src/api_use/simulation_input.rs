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

#[pyclass]
pub struct SimulationInput;

#[pymethods]
impl SimulationInput {
    #[staticmethod]
    pub fn new(
        uuid: String,
        directory: String,
        input_header: &str,
        max_bytes_on_disk: u64,
    ) -> Result<Self> {
        try_with_context(move |context| {
            context.new_simulation_input(
                uuid,
                directory.into(),
                from_str(input_header).context("Input header string isn't valid JSON")?,
                max_bytes_on_disk,
            )?;
            Ok(Self)
        })
    }

    pub fn start_frame(&self, frame_start: &str) -> Result<()> {
        try_with_context(|context| {
            context
                .get_simulation_input()
                .context("Not recording input")?
                .start_frame(from_str(frame_start).context("Frame start string isn't valid JSON")?)
        })
    }

    pub fn record_input_float<'py>(
        &self,
        meta: &str,
        bulk: PyReadonlyArray1<'py, f32>,
    ) -> Result<()> {
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

    pub fn record_input_int<'py>(
        &self,
        meta: &str,
        bulk: PyReadonlyArray1<'py, i32>,
    ) -> Result<()> {
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

    pub fn finish_frame(&self) -> Result<()> {
        try_with_context(|context| {
            context
                .get_simulation_input()
                .context("Not recording input")?
                .finish_frame()
        })
    }
}
