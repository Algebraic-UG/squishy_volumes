// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::BTreeMap;

use anyhow::{Result, ensure};
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;

use crate::{
    input_file::{InputFrame, InputReader},
    profile,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct InterpolatedInput {
    pub gravity: Vector3<T>,
    pub particles_input: BTreeMap<String, InterpolatedInputParticles>,
    pub collider_input: BTreeMap<String, InterpolatedInputCollider>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InterpolatedInputParticles {
    pub goal_positions: Vec<Vector3<T>>,
    pub goal_stiffnesses: Vec<T>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InterpolatedInputCollider {
    pub vertex_positions: Vec<Vector3<T>>,
    pub vertex_velocities: Vec<Vector3<T>>,
    pub triangles: Vec<[u32; 3]>,
}

pub struct InputInterpolationPoint {
    pub frame: usize,
    pub input_frame: InputFrame,
}

pub struct InputInterpolation {
    input_reader: InputReader,
    a: Option<InputInterpolationPoint>,
    b: Option<InputInterpolationPoint>,
}

impl InputInterpolation {
    pub fn new(input_reader: InputReader) -> Result<Self> {
        ensure!(input_reader.len() > 0);
        Ok(Self {
            input_reader,
            a: None,
            b: None,
        })
    }

    pub fn load(&mut self, frame: usize) -> Result<()> {
        profile!("load interpolants");

        let max_frame = self.input_reader.len() - 1;

        // if we're too far, just use the last available and skip b
        let frame = frame.min(max_frame);

        // a already corret, so b must be as well (could be none)
        if self.a.as_ref().is_some_and(|point| point.frame == frame) {
            return Ok(());
        }

        // always load something into a
        // could be that we already have the next in b
        if let Some(b) = self.b.take()
            && b.frame == frame
        {
            self.a = Some(b);
        } else {
            let input_frame = self.input_reader.read_frame(frame)?;
            self.a = Some(InputInterpolationPoint { frame, input_frame })
        }

        // might skip b
        if frame == max_frame {
            return Ok(());
        }

        // load b
        let frame = frame + 1;
        let input_frame = self.input_reader.read_frame(frame)?;
        self.b = Some(InputInterpolationPoint { frame, input_frame });

        Ok(())
    }

    pub fn a(&self) -> Option<&InputInterpolationPoint> {
        self.a.as_ref()
    }

    pub fn b(&self) -> Option<&InputInterpolationPoint> {
        self.b.as_ref()
    }
}
