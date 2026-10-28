// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Context, Result};
use numpy::PyArray1;
use pyo3::{prelude::*, types::PyList};
use serde_json::{from_str, to_string};

use crate::hot_reloadable::{try_with_context, with_context};

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
            )
            .unwrap())
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

    pub fn computing(&self) -> Result<bool> {
        with_context(|context| {
            context
                .get_simulation(&self.0)
                .is_some_and(|simulation| simulation.computing())
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
                .pause_compute();
            Ok(())
        })
    }

    pub fn available_frames(&self) -> Result<usize> {
        with_context(|context| {
            context
                .get_simulation(&self.0)
                .map_or(0, |simulation| simulation.available_frames())
        })
    }

    pub fn stats(&self, frame: usize) -> Result<String> {
        try_with_context(|context| {
            Ok(to_string(
                &context
                    .get_simulation(&self.0)
                    .context("No simulation found for {self.0}")?
                    .stats(frame)?,
            )?)
        })
    }

    pub fn drop(&self) -> Result<()> {
        with_context(move |context| context.drop_simulation(&self.0))
    }
}
