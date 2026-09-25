// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::borrow::Cow;

use anyhow::Result;
use serde_json::Value;

pub trait SimulationInput {
    fn start_frame(&mut self, frame_start: Value) -> Result<()>;
    fn record_input(&mut self, meta: Value, bulk: InputBulk) -> Result<()>;
    fn finish_frame(&mut self) -> Result<()>;
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub enum InputBulk<'a> {
    Bool(Cow<'a, [bool]>),
    Floats(Cow<'a, [f32]>),
    Ints(Cow<'a, [i32]>),
}

impl InputBulk<'_> {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn len(&self) -> usize {
        match self {
            InputBulk::Bool(cow) => cow.len(),
            InputBulk::Floats(cow) => cow.len(),
            InputBulk::Ints(cow) => cow.len(),
        }
    }
}
