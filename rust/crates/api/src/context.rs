// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

use crate::{Simulation, SimulationInput};

pub trait Context: Send + Sync {
    fn new_simulation_input(
        &mut self,
        uuid: String,
        directory: PathBuf,
        input_header: Value,
        max_bytes_on_disk: u64,
    ) -> Result<()>;

    fn get_simulation_input(&mut self) -> Option<&mut dyn SimulationInput>;

    fn new_simulation(&mut self) -> Result<()>;
    fn load_simulation(&mut self, uuid: String, directory: PathBuf) -> Result<()>;

    fn get_simulation(&self, uuid: &str) -> Option<&dyn Simulation>;
    fn get_simulation_mut(&mut self, uuid: &str) -> Option<&mut dyn Simulation>;

    fn drop_simulation(&mut self, uuid: &str);
}
