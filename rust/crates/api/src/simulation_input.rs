// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Result, bail};
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

impl InputBulk<'_> {
    pub fn len(&self) -> usize {
        match self {
            InputBulk::F32(slice) => slice.len(),
            InputBulk::I32(slice) => slice.len(),
        }
    }
}

impl TryFrom<InputBulk<'_>> for Vec<f32> {
    type Error = anyhow::Error;

    fn try_from(input_bulk: InputBulk<'_>) -> std::result::Result<Self, Self::Error> {
        let InputBulk::F32(slice) = input_bulk else {
            bail!("input bulk should be floats");
        };
        Ok(Self::from(slice))
    }
}

impl TryFrom<InputBulk<'_>> for Vec<i32> {
    type Error = anyhow::Error;

    fn try_from(input_bulk: InputBulk<'_>) -> std::result::Result<Self, Self::Error> {
        let InputBulk::I32(slice) = input_bulk else {
            bail!("input bulk should be ints");
        };
        Ok(Self::from(slice))
    }
}
