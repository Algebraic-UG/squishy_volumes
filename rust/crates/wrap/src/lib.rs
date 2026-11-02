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

mod hot_reloadable;
use hot_reloadable::{initialize, CombinedBuildInfo};

#[cfg(feature = "hot_reload")]
use hot_reloadable::handle_reload;

mod api_use;
use crate::api_use::{available_gpus, simulation::Simulation, simulation_input::SimulationInput};

fn squishy_volumes_wrap(m: &Bound<'_, PyModule>) -> PyResult<()> {
    initialize();

    #[cfg(feature = "hot_reload")]
    handle_reload();

    m.add_function(wrap_pyfunction!(build_info_as_json, m)?)?;
    m.add_function(wrap_pyfunction!(available_gpus, m)?)?;

    m.add_class::<Simulation>()?;
    m.add_class::<SimulationInput>()?;

    Ok(())
}

#[pyfunction]
fn build_info_as_json() -> String {
    to_string(&CombinedBuildInfo::new()).unwrap()
}
