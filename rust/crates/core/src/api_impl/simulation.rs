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

impl squishy_volumes_api::Simulation for crate::SimulationImpl {
    fn input_header(&self) -> anyhow::Result<serde_json::Value> {
        Ok(self.input_header_impl()?)
    }

    fn computing(&self) -> bool {
        self.computing_impl()
    }

    fn poll(&mut self) -> anyhow::Result<serde_json::Value> {
        Ok(self.poll_impl()?)
    }

    fn start_compute(&mut self, compute_settings: serde_json::Value) -> anyhow::Result<()> {
        Ok(self.start_compute_impl(compute_settings)?)
    }

    fn pause_compute(&mut self) {
        self.pause_compute_impl()
    }

    fn available_frames(&self) -> usize {
        self.available_frames_impl()
    }
}
