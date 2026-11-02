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

use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

use crate::{Simulation, SimulationInput};

pub trait Context: Send + Sync {
    fn available_gpus(&self) -> Vec<String>;

    fn new_simulation_input(
        &mut self,
        uuid: String,
        directory: PathBuf,
        input_header: Value,
        max_bytes_on_disk: u64,
    ) -> Result<()>;

    fn get_simulation_input(&mut self) -> Option<&mut dyn SimulationInput>;
    fn drop_simulation_input(&mut self);

    fn new_simulation(&mut self) -> Result<String>;
    fn load_simulation(&mut self, uuid: String, directory: PathBuf) -> Result<()>;

    fn get_simulation(&self, uuid: &str) -> Option<&dyn Simulation>;
    fn get_simulation_mut(&mut self, uuid: &str) -> Option<&mut dyn Simulation>;

    fn drop_simulation(&mut self, uuid: &str);
}
