// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

impl squishy_volumes_api::Context for crate::ContextImpl {
    fn available_gpus(&self) -> Vec<String> {
        squishy_volumes_gpu::GpuContext::available_gpus()
    }

    fn new_simulation_input(
        &mut self,
        uuid: String,
        directory: std::path::PathBuf,
        input_header: serde_json::Value,
        max_bytes_on_disk: u64,
    ) -> anyhow::Result<()> {
        Ok(self.new_simulation_input_impl(uuid, directory, input_header, max_bytes_on_disk)?)
    }

    fn get_simulation_input(&mut self) -> Option<&mut dyn squishy_volumes_api::SimulationInput> {
        self.get_simulation_input_impl()
    }

    fn drop_simulation_input(&mut self) {
        self.drop_simulation_input_impl()
    }

    fn new_simulation(&mut self) -> anyhow::Result<String> {
        Ok(self.new_simulation_impl()?)
    }

    fn load_simulation(
        &mut self,
        uuid: String,
        directory: std::path::PathBuf,
    ) -> anyhow::Result<()> {
        Ok(self.load_simulation_impl(uuid, directory)?)
    }

    fn get_simulation(&self, uuid: &str) -> Option<&dyn squishy_volumes_api::Simulation> {
        self.get_simulation_impl(uuid)
    }

    fn get_simulation_mut(
        &mut self,
        uuid: &str,
    ) -> Option<&mut dyn squishy_volumes_api::Simulation> {
        self.get_simulation_mut_impl(uuid)
    }

    fn drop_simulation(&mut self, uuid: &str) {
        self.drop_simulation_impl(uuid)
    }
}
