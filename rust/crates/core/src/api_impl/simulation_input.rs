// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

impl squishy_volumes_api::SimulationInput for crate::SimulationInputImpl {
    fn start_frame(&mut self, frame_start: serde_json::Value) -> anyhow::Result<()> {
        Ok(self.start_frame_impl(frame_start)?)
    }

    fn record_input(
        &mut self,
        meta: serde_json::Value,
        bulk: squishy_volumes_api::InputBulk,
    ) -> anyhow::Result<()> {
        Ok(self.record_input_impl(meta, bulk)?)
    }

    fn finish_frame(&mut self) -> anyhow::Result<()> {
        Ok(self.finish_frame_impl()?)
    }
}
