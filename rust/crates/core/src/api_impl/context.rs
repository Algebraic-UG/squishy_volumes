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

use std::path::PathBuf;

use serde_json::Value;
use squishy_volumes_api::{Simulation, SimulationInput};

use crate::{ContextImpl, guard::guard};

impl squishy_volumes_api::Context for ContextImpl {
    fn available_gpus(&self) -> anyhow::Result<Vec<String>> {
        Ok(guard(|| {
            Ok(squishy_volumes_gpu::GpuContext::available_gpus())
        })?)
    }

    fn new_simulation_input(
        &mut self,
        uuid: String,
        directory: PathBuf,
        input_header: Value,
        max_bytes_on_disk: u64,
    ) -> anyhow::Result<()> {
        Ok(guard(|| {
            self.new_simulation_input_impl(uuid, directory, input_header, max_bytes_on_disk)
        })?)
    }

    fn get_simulation_input(&mut self) -> Option<&mut dyn SimulationInput> {
        self.get_simulation_input_impl()
    }

    fn drop_simulation_input(&mut self) -> anyhow::Result<()> {
        Ok(guard(|| {
            self.drop_simulation_input_impl();
            Ok(())
        })?)
    }

    fn new_simulation(&mut self) -> anyhow::Result<String> {
        Ok(guard(|| self.new_simulation_impl())?)
    }

    fn load_simulation(&mut self, uuid: String, directory: PathBuf) -> anyhow::Result<()> {
        Ok(guard(|| self.load_simulation_impl(uuid, directory))?)
    }

    fn get_simulation(&self, uuid: &str) -> Option<&dyn Simulation> {
        self.get_simulation_impl(uuid)
    }

    fn get_simulation_mut(&mut self, uuid: &str) -> Option<&mut dyn Simulation> {
        self.get_simulation_mut_impl(uuid)
    }

    fn drop_simulation(&mut self, uuid: &str) -> anyhow::Result<()> {
        Ok(guard(|| {
            self.drop_simulation_impl(uuid);
            Ok(())
        })?)
    }
}
