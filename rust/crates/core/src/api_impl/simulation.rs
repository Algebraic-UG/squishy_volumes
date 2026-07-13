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

    fn pause_compute(&mut self) -> anyhow::Result<()> {
        Ok(self.pause_compute_impl()?)
    }

    fn available_frames(&self) -> usize {
        self.available_frames_impl()
    }

    fn available_attributes(&self) -> anyhow::Result<Vec<serde_json::Value>> {
        Ok(self.available_attributes_impl()?)
    }

    fn fetch_flat_attribute_f32(
        &self,
        frame: usize,
        attribute: serde_json::Value,
    ) -> anyhow::Result<Vec<f32>> {
        Ok(self.fetch_flat_attribute_f32_impl(frame, attribute)?)
    }

    fn fetch_flat_attribute_i32(
        &self,
        frame: usize,
        attribute: serde_json::Value,
    ) -> anyhow::Result<Vec<i32>> {
        Ok(self.fetch_flat_attribute_i32_impl(frame, attribute)?)
    }

    fn stats(&self) -> anyhow::Result<serde_json::Value> {
        Ok(self.stats_impl()?)
    }
}
