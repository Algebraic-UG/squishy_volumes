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
use crate::api_use::{context, simulation, simulation_input};

fn squishy_volumes_wrap(m: &Bound<'_, PyModule>) -> PyResult<()> {
    initialize();

    #[cfg(feature = "hot_reload")]
    handle_reload();

    m.add_function(wrap_pyfunction!(build_info_as_json, m)?)?;

    m.add_function(wrap_pyfunction!(context::new_simulation_input, m)?)?;
    m.add_function(wrap_pyfunction!(context::new_simulation, m)?)?;
    m.add_function(wrap_pyfunction!(context::load_simulation, m)?)?;
    m.add_function(wrap_pyfunction!(context::drop_simulation, m)?)?;

    m.add_function(wrap_pyfunction!(simulation::poll, m)?)?;
    m.add_function(wrap_pyfunction!(simulation::computing, m)?)?;
    m.add_function(wrap_pyfunction!(simulation::start_compute, m)?)?;
    m.add_function(wrap_pyfunction!(simulation::pause_compute, m)?)?;
    m.add_function(wrap_pyfunction!(simulation::available_frames, m)?)?;
    m.add_function(wrap_pyfunction!(simulation::available_attributes, m)?)?;
    m.add_function(wrap_pyfunction!(simulation::fetch_flat_attribute, m)?)?;
    m.add_function(wrap_pyfunction!(simulation::stats, m)?)?;

    m.add_function(wrap_pyfunction!(simulation_input::start_frame, m)?)?;
    m.add_function(wrap_pyfunction!(simulation_input::record_input_float, m)?)?;
    m.add_function(wrap_pyfunction!(simulation_input::record_input_int, m)?)?;
    m.add_function(wrap_pyfunction!(simulation_input::finish_frame, m)?)?;

    Ok(())
}

#[pyfunction]
fn build_info_as_json() -> String {
    to_string(&CombinedBuildInfo::new()).unwrap()
}
