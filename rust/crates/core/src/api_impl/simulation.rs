// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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
