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
use squishy_volumes_api::ComputeSettings;

use crate::hot_reloadable::{try_with_context, with_context};

#[pyfunction]
pub fn poll(uuid: &str) -> Result<String> {
    try_with_context(|context| {
        Ok(context
            .get_simulation_mut(uuid)
            .context("No simulation found")?
            .poll()?
            .map(|task| to_string(&task).unwrap())
            .unwrap_or_default())
    })
}

#[pyfunction]
pub fn computing(uuid: &str) -> Result<bool> {
    with_context(|context| {
        context
            .get_simulation(uuid)
            .is_some_and(|simulation| simulation.computing())
    })
}

// TODO: not sure how to improve this (too_many_arguments)
// might need another #[pyclass]
#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn start_compute(
    uuid: &str,
    time_step: f32,
    explicit: bool,
    debug_mode: bool,
    adaptive_time_steps: bool,
    next_frame: usize,
    number_of_frames: usize,
    max_bytes_on_disk: u64,
) -> Result<()> {
    try_with_context(|context| {
        context
            .get_simulation_mut(uuid)
            .context("No simulation found")?
            .start_compute(ComputeSettings {
                time_step,
                explicit,
                debug_mode,
                adaptive_time_steps,
                next_frame,
                number_of_frames,
                max_bytes_on_disk,
            })
    })
}

#[pyfunction]
pub fn pause_compute(uuid: &str) -> Result<()> {
    try_with_context(|context| {
        context
            .get_simulation_mut(uuid)
            .context("No simulation found")?
            .pause_compute();
        Ok(())
    })
}

#[pyfunction]
pub fn available_frames(uuid: &str) -> Result<usize> {
    with_context(|context| {
        context
            .get_simulation(uuid)
            .map_or(0, |simulation| simulation.available_frames())
    })
}

#[pyfunction]
pub fn available_attributes<'py>(
    uuid: &str,
    py: Python<'py>,
    frame: usize,
) -> Result<Bound<'py, PyList>> {
    try_with_context(|context| {
        let attributes = context
            .get_simulation(uuid)
            .context("No simulation found")?
            .available_attributes(frame)?
            .into_iter()
            .map(|attribute| Ok(to_string(&attribute)?))
            .collect::<Result<Vec<_>>>()?;
        Ok(PyList::new(py, attributes)?)
    })
}

#[pyfunction]
pub fn fetch_flat_attribute<'py>(
    uuid: &str,
    py: Python<'py>,
    frame: usize,
    attribute: &str,
) -> Result<Bound<'py, PyArray1<f32>>> {
    try_with_context(|context| {
        let flat_attribute = context
            .get_simulation(uuid)
            .context("No simulation found")?
            .fetch_flat_attribute(
                frame,
                from_str(attribute).context("Attribute string isn't valid json")?,
            )?;
        Ok(PyArray1::from_vec(py, flat_attribute))
    })
}

#[pyfunction]
pub fn stats(uuid: &str) -> Result<String> {
    try_with_context(|context| {
        Ok(to_string(
            &context
                .get_simulation(uuid)
                .context("No simulation found")?
                .stats()?,
        )?)
    })
}
