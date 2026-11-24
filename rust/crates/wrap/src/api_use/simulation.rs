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

use anyhow::{Context, Result};
use pyo3::prelude::*;
use serde_json::{from_str, to_string};

use crate::try_with_context;

#[pyclass]
pub struct Simulation(pub String);

#[pymethods]
impl Simulation {
    #[staticmethod]
    pub fn new() -> Result<Self> {
        try_with_context(|context| {
            let uuid = context.new_simulation()?;
            Ok(Self(uuid))
        })
    }

    #[staticmethod]
    #[pyo3(signature = (*, uuid, directory))]
    pub fn load(uuid: String, directory: String) -> Result<Self> {
        try_with_context(move |context| {
            context.load_simulation(uuid.clone(), directory.into())?;
            Ok(Self(uuid))
        })
    }

    pub fn uuid(&self) -> String {
        self.0.clone()
    }

    pub fn input_header(&self) -> Result<String> {
        try_with_context(|context| {
            Ok(to_string(
                &context
                    .get_simulation(&self.0)
                    .with_context(|| format!("No simulation found for {}", self.0))?
                    .input_header()?,
            )?)
        })
    }

    pub fn poll(&self) -> Result<String> {
        try_with_context(|context| {
            Ok(to_string(
                &context
                    .get_simulation_mut(&self.0)
                    .with_context(|| format!("No simulation found for {}", self.0))?
                    .poll()?,
            )?)
        })
    }

    pub fn baking(&self) -> Result<bool> {
        try_with_context(|context| {
            context
                .get_simulation(&self.0)
                .map_or(Ok(false), |simulation| simulation.baking())
        })
    }

    #[pyo3(signature = (*, compute_settings))]
    pub fn start_compute(&self, compute_settings: &str) -> Result<()> {
        try_with_context(|context| {
            context
                .get_simulation_mut(&self.0)
                .with_context(|| format!("No simulation found for {}", self.0))?
                .start_compute(
                    from_str(compute_settings)
                        .context("Compute settings string isn't valid json")?,
                )
        })
    }

    pub fn pause_compute(&self) -> Result<()> {
        try_with_context(|context| {
            context
                .get_simulation_mut(&self.0)
                .with_context(|| format!("No simulation found for {}", self.0))?
                .pause_compute()
        })
    }

    pub fn available_frames(&self) -> Result<usize> {
        try_with_context(|context| {
            context
                .get_simulation(&self.0)
                .map_or(Ok(0), |simulation| simulation.available_frames())
        })
    }

    pub fn drop(&self) -> Result<()> {
        try_with_context(move |context| context.drop_simulation(&self.0))
    }
}
