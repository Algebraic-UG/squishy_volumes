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

use std::sync::OnceLock;

use serde_json::Value;

use crate::{
    Error, SimulationImpl,
    guard::{Breakable, guard, guard_breakable},
};

impl Breakable for SimulationImpl {
    fn panic_message(&self) -> &OnceLock<String> {
        &self.panic_message
    }

    fn broken(panic_message: String) -> Error {
        Error::BrokenSimulation(panic_message)
    }
}

#[allow(clippy::result_large_err)]
impl squishy_volumes_api::Simulation for SimulationImpl {
    fn input_header(&self) -> anyhow::Result<Value> {
        Ok(guard_breakable(self, |simulation| {
            simulation.input_header_impl()
        })?)
    }

    fn baking(&self) -> anyhow::Result<bool> {
        Ok(guard_breakable(self, |simulation| {
            Ok(simulation.baking_impl())
        })?)
    }

    fn poll(&mut self) -> anyhow::Result<Value> {
        Ok(guard_breakable(self, |simulation| simulation.poll_impl())?)
    }

    fn start_compute(&mut self, compute_settings: Value) -> anyhow::Result<()> {
        Ok(guard_breakable(self, |simulation| {
            simulation.start_compute_impl(compute_settings)
        })?)
    }

    fn pause_compute(&mut self) -> anyhow::Result<()> {
        Ok(guard(|| self.pause_compute_impl())?)
    }

    fn available_frames(&self) -> anyhow::Result<usize> {
        Ok(guard_breakable(self, |simulation| {
            Ok(simulation.available_frames_impl())
        })?)
    }
}
