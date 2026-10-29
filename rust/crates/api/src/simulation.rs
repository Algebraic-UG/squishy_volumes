// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use serde_json::Value;

pub trait Simulation {
    fn input_header(&self) -> Result<Value>;

    fn computing(&self) -> bool;

    fn poll(&mut self) -> Result<Value>;

    fn start_compute(&mut self, settings: Value) -> Result<()>;
    fn pause_compute(&mut self);

    fn available_frames(&self) -> usize;
}
