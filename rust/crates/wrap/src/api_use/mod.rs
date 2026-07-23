// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use pyo3::{prelude::*, types::PyList};

pub mod simulation;
pub mod simulation_input;

#[pyfunction]
pub fn available_gpus<'py>(py: Python<'py>) -> Result<Bound<'py, PyList>> {
    let gpus: Vec<String> =
        crate::hot_reloadable::with_context(|context| context.available_gpus())?;
    Ok(PyList::new(py, gpus)?)
}
