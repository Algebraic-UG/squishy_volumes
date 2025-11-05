// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{bail, Context, Result};
use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::{
    prelude::*,
    types::{PyDict, PyList},
};
use serde_json::{from_str, to_string};
use squishy_volumes_api::{ComputeSettings, InputBulk};
use std::{collections::BTreeMap, path::PathBuf};

mod shim;
pub use shim::*;

mod hot_reloadable;
use hot_reloadable::{initialize, try_with_context, with_context, CombinedBuildInfo};

#[cfg(feature = "hot_reload")]
use hot_reloadable::handle_reload;

fn squishy_volumes_wrap(m: &Bound<'_, PyModule>) -> PyResult<()> {
    initialize();

    #[cfg(feature = "hot_reload")]
    handle_reload();

    m.add_function(wrap_pyfunction!(build_info_as_json, m)?)?;
    m.add_class::<SimulationReference>()?;
    m.add_function(wrap_pyfunction!(new, m)?)?;
    m.add_function(wrap_pyfunction!(load, m)?)?;

    Ok(())
}

#[pyfunction]
fn build_info_as_json() -> String {
    to_string(&CombinedBuildInfo::new()).unwrap()
}

#[pyclass]
struct SimulationReference(String);

#[pyfunction]
fn new(
    uuid: String,
    cache_dir: String,
    json: &str,
    max_bytes_on_disk: u64,
) -> Result<SimulationReference> {
    try_with_context(move |context| {
        context.new_simulation(
            uuid.clone(),
            PathBuf::from(cache_dir),
            from_str(json).context("Setup string isn't valid JSON")?,
            max_bytes_on_disk,
        )?;
        Ok(SimulationReference(uuid))
    })
}

#[pyfunction]
fn load(uuid: String, cache_dir: String, max_bytes_on_disk: u64) -> Result<SimulationReference> {
    try_with_context(|context| {
        context.load_simulation(uuid.clone(), cache_dir.into(), max_bytes_on_disk)?;
        Ok(SimulationReference(uuid))
    })
}

#[pymethods]
impl SimulationReference {
    fn record_input<'py>(&self, frame: usize, dict: &Bound<'py, PyDict>) -> Result<()> {
        enum Array<'py> {
            F32(PyReadonlyArray1<'py, f32>),
            I32(PyReadonlyArray1<'py, i32>),
        }

        try_with_context(|context| {
            let mut keys = Vec::new();
            let mut arrays = Vec::new();
            for (key, value) in dict.iter() {
                keys.push(key.extract()?);
                if let Ok(array) = value.extract::<PyReadonlyArray1<f32>>() {
                    arrays.push(Array::F32(array));
                    continue;
                }
                if let Ok(array) = value.extract::<PyReadonlyArray1<i32>>() {
                    arrays.push(Array::I32(array));
                    continue;
                }
                bail!(
                    "Expected numpy array, 1D, float32 or int32, but got: {}",
                    value.get_type()
                )
            }
            let bulk = keys
                .into_iter()
                .zip(&arrays)
                .map(|(key, array)| {
                    Ok((
                        key,
                        match array {
                            Array::F32(array) => InputBulk::F32(array.as_slice()?),
                            Array::I32(array) => InputBulk::I32(array.as_slice()?),
                        },
                    ))
                })
                .collect::<Result<BTreeMap<String, InputBulk>>>()?;
            context
                .get_simulation_mut(&self.0)?
                .record_input(frame, bulk)
        })
    }

    fn drop(&self) -> Result<()> {
        try_with_context(|context| context.drop_simulation(&self.0))
    }

    fn poll(&self) -> Result<String> {
        try_with_context(|context| {
            Ok(context
                .get_simulation_mut(&self.0)?
                .poll()?
                .map(|task| to_string(&task).unwrap())
                .unwrap_or_default())
        })
    }

    fn computing(&self) -> Result<bool> {
        with_context(|context| {
            context
                .get_simulation(&self.0)
                .is_ok_and(|simulation| simulation.computing())
        })
    }

    // TODO: not sure how to improve this (too_many_arguments)
    // might need another #[pyclass]
    #[allow(clippy::too_many_arguments)]
    fn start_compute(
        &self,
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
                .get_simulation_mut(&self.0)?
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

    fn pause_compute(&self) -> Result<()> {
        try_with_context(|context| {
            context.get_simulation_mut(&self.0)?.pause_compute();
            Ok(())
        })
    }

    fn available_frames(&self) -> Result<usize> {
        with_context(|context| {
            context
                .get_simulation(&self.0)
                .map_or(0, |simulation| simulation.available_frames())
        })
    }

    fn available_attributes<'py>(
        &self,
        py: Python<'py>,
        frame: usize,
    ) -> Result<Bound<'py, PyList>> {
        try_with_context(|context| {
            let attributes = context
                .get_simulation(&self.0)?
                .available_attributes(frame)?
                .into_iter()
                .map(|attribute| Ok(to_string(&attribute)?))
                .collect::<Result<Vec<_>>>()?;
            Ok(PyList::new(py, attributes)?)
        })
    }

    fn fetch_flat_attribute<'py>(
        &self,
        py: Python<'py>,
        frame: usize,
        attribute: &str,
    ) -> Result<Bound<'py, PyArray1<f32>>> {
        try_with_context(|context| {
            let flat_attribute = context.get_simulation(&self.0)?.fetch_flat_attribute(
                frame,
                from_str(attribute).context("Attribute string isn't valid json")?,
            )?;
            Ok(PyArray1::from_vec(py, flat_attribute))
        })
    }

    fn stats(&self) -> Result<String> {
        try_with_context(|context| Ok(to_string(&context.get_simulation(&self.0)?.stats()?)?))
    }
}
