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

use std::borrow::Cow;

use anyhow::{Context, Result};
use numpy::PyReadonlyArray1;
use pyo3::prelude::*;
use serde_json::from_str;
use squishy_volumes_api::InputBulk;

use crate::hot_reloadable::{try_with_context, with_context};

#[pyclass]
pub struct SimulationInput;

#[pymethods]
impl SimulationInput {
    #[staticmethod]
    #[pyo3(signature = (*, uuid, directory, input_header, max_bytes_on_disk))]
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

    #[pyo3(signature = (*, frame_start))]
    pub fn start_frame(&self, frame_start: &str) -> Result<()> {
        try_with_context(|context| {
            context
                .get_simulation_input()
                .context("Not recording input")?
                .start_frame(from_str(frame_start).context("Frame start string isn't valid JSON")?)
        })
    }

    #[pyo3(signature = (*, meta, bulk))]
    pub fn record_input_bool<'py>(
        &self,
        meta: &str,
        bulk: PyReadonlyArray1<'py, bool>,
    ) -> Result<()> {
        try_with_context(|context| {
            context
                .get_simulation_input()
                .context("Not recording input")?
                .record_input(
                    from_str(meta).context("Meta string isn't valid JSON")?,
                    InputBulk::Bool(Cow::Borrowed(bulk.as_slice()?)),
                )
        })
    }

    #[pyo3(signature = (*, meta, bulk))]
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
                    InputBulk::Floats(Cow::Borrowed(bulk.as_slice()?)),
                )
        })
    }

    #[pyo3(signature = (*, meta, bulk))]
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
                    InputBulk::Ints(Cow::Borrowed(bulk.as_slice()?)),
                )
        })
    }

    pub fn drop(&self) -> Result<()> {
        with_context(|context| context.drop_simulation_input())
    }
}
