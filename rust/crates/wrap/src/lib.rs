// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use pyo3::prelude::*;
use serde_json::to_string;

mod shim;
pub use shim::*;

mod hot_reloadable;
use hot_reloadable::{initialize, CombinedBuildInfo};

#[cfg(feature = "hot_reload")]
use hot_reloadable::handle_reload;

mod api_use;
use crate::api_use::{simulation::Simulation, simulation_input::SimulationInput};

fn squishy_volumes_wrap(m: &Bound<'_, PyModule>) -> PyResult<()> {
    initialize();

    #[cfg(feature = "hot_reload")]
    handle_reload();

    m.add_function(wrap_pyfunction!(build_info_as_json, m)?)?;

    m.add_class::<Simulation>()?;
    m.add_class::<SimulationInput>()?;

    Ok(())
}

#[pyfunction]
fn build_info_as_json() -> String {
    to_string(&CombinedBuildInfo::new()).unwrap()
}
