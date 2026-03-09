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
    Bool(&'a [bool]),
    F32(&'a [f32]),
    I32(&'a [i32]),
}

impl InputBulk<'_> {
    pub fn len(&self) -> usize {
        match self {
            InputBulk::Bool(slice) => slice.len(),
            InputBulk::F32(slice) => slice.len(),
            InputBulk::I32(slice) => slice.len(),
        }
    }

    pub fn as_bools(&self) -> Result<&[bool]> {
        let InputBulk::Bool(slice) = self else {
            bail!("input bulk should be bools");
        };
        Ok(slice)
    }

    pub fn as_floats(&self) -> Result<&[f32]> {
        let InputBulk::F32(slice) = self else {
            bail!("input bulk should be floats");
        };
        Ok(slice)
    }

    pub fn as_ints(&self) -> Result<&[i32]> {
        let InputBulk::I32(slice) = self else {
            bail!("input bulk should be ints");
        };
        Ok(slice)
    }
}

impl TryFrom<InputBulk<'_>> for Vec<bool> {
    type Error = anyhow::Error;

    fn try_from(input_bulk: InputBulk<'_>) -> std::result::Result<Self, Self::Error> {
        Ok(Self::from(input_bulk.as_bools()?))
    }
}

impl TryFrom<InputBulk<'_>> for Vec<f32> {
    type Error = anyhow::Error;

    fn try_from(input_bulk: InputBulk<'_>) -> std::result::Result<Self, Self::Error> {
        Ok(Self::from(input_bulk.as_floats()?))
    }
}

impl TryFrom<InputBulk<'_>> for Vec<i32> {
    type Error = anyhow::Error;

    fn try_from(input_bulk: InputBulk<'_>) -> std::result::Result<Self, Self::Error> {
        Ok(Self::from(input_bulk.as_ints()?))
    }
}
