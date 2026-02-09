// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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
}

struct InputInterpolationPoint {
    frame: usize,
    input_frame: InputFrame,
}

pub struct InputInterpolation {
    seconds_per_frame: f64,
    input_reader: InputReader,
    a: Option<InputInterpolationPoint>,
    b: Option<InputInterpolationPoint>,
}

impl InputInterpolation {
    pub fn new(seconds_per_frame: f64, input_reader: InputReader) -> Result<Self> {
        ensure!(input_reader.len() > 0);
        Ok(Self {
            seconds_per_frame,
            input_reader,
            a: None,
            b: None,
        })
    }

    pub fn interpolate(&mut self, time: f64) -> Result<InterpolatedInput> {
        profile!("interpolate");

        // this should be a no-op for all in-between-frame-steps
        self.load((time / self.seconds_per_frame).floor() as usize)?;

        let a = self
            .a
            .as_ref()
            .map(|point| &point.input_frame)
            .expect("there's always the a point");

        // in this case assume a constant extrapolation from a
        let Some(b) = self.b.as_ref().map(|point| &point.input_frame) else {
            let gravity = a.gravity;
            return Ok(InterpolatedInput { gravity });
        };

        // linear interpolation between a and b
        let factor_b = ((time / self.seconds_per_frame) % 1.) as f32;
        let factor_a = 1. - factor_b;

        let gravity = factor_a * a.gravity + factor_b * b.gravity;

        Ok(InterpolatedInput { gravity })
    }

    fn load(&mut self, frame: usize) -> Result<()> {
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
}
