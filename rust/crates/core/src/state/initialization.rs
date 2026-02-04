// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    collections::BTreeMap,
    num::NonZero,
    sync::{Arc, atomic::AtomicBool},
};

use thiserror::Error;

use crate::{
    Report, ReportInfo,
    input_file::{InputFrame, InputHeader, InputObjectType},
    state::particles::Particles,
};

use super::State;

#[derive(Error, Debug)]
pub enum StateInitializationError {}

impl State {
    pub fn new(
        run: Arc<AtomicBool>,
        report: Report,
        input_header: InputHeader,
        first_frame: InputFrame,
    ) -> Result<Self, StateInitializationError> {
        let report = report.new_sub(ReportInfo {
            name: "Initializing Objects".to_string(),
            completed_steps: 0,
            steps_to_completion: NonZero::new(input_header.objects.len().max(1)).unwrap(),
        });

        let mut name_map = BTreeMap::new();
        let mut particles = Particles::default();
        let mut particle_objects = Vec::new();

        for object in input_header.objects {
            match object.ty {
                InputObjectType::Particles => {}
            }

            report.step();
        }

        let time = 0.;
        let phase = Default::default();

        let grid_momentum = Default::default();
        let grid_collider_distances = Default::default();
        let grid_collider_momentums = Default::default();

        Ok(Self {
            time,
            phase,
            name_map,
            particle_objects,
            particles,
            grid_momentum,
            grid_collider_distances,
            grid_collider_momentums,
        })
    }
}
