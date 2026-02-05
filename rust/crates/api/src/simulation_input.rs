// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use serde_json::Value;

pub trait SimulationInput {
    fn start_frame(&mut self, frame_start: Value) -> Result<()>;
    fn record_input(&mut self, meta: Value, bulk: InputBulk) -> Result<()>;
    fn finish_frame(&mut self) -> Result<()>;
}

#[derive(Debug)]
pub enum InputBulk<'a> {
    F32(&'a [f32]),
    I32(&'a [i32]),
}
