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

use pyo3::prelude::*;
use serde_json::to_string;

mod shim;
mod tests;
pub use shim::*;

#[cfg(feature = "hot_reload")]
mod hot_reload;
#[cfg(feature = "hot_reload")]
use hot_reload as context;

#[cfg(not(feature = "hot_reload"))]
mod static_context;
#[cfg(not(feature = "hot_reload"))]
use static_context as context;

use context::{core_build_info, initialize, try_with_context};

mod api_use;
use crate::api_use::{available_gpus, simulation::Simulation, simulation_input::SimulationInput};

fn squishy_volumes_wrap(m: &Bound<'_, PyModule>) -> PyResult<()> {
    initialize();

    #[cfg(feature = "hot_reload")]
    hot_reload::handle_reload();

    m.add_function(wrap_pyfunction!(build_info_as_json, m)?)?;
    m.add_function(wrap_pyfunction!(available_gpus, m)?)?;

    m.add_class::<Simulation>()?;
    m.add_class::<SimulationInput>()?;

    Ok(())
}

const BUG: &str = "You encountered a bug in Squishy Volumes.
It's unlikely that the process can recover.
Please consider restarting the application.";

#[derive(serde::Serialize)]
pub struct CombinedBuildInfo {
    pub wrapper: build_info::BuildInfo,
    pub core_libs: build_info::BuildInfo,
}

build_info::build_info!(fn _build_info);
impl CombinedBuildInfo {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            wrapper: _build_info().clone(),
            core_libs: core_build_info()?,
        })
    }
}

#[pyfunction]
fn build_info_as_json() -> anyhow::Result<String> {
    Ok(to_string(&CombinedBuildInfo::new()?)?)
}
