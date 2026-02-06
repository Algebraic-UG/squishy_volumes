// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value};
use squishy_volumes_api::{InputBulk, SimulationInput, T};
use tracing::info;

use crate::{
    directory_lock::DirectoryLock,
    input_file::{InputFrame, InputHeader, InputObjectType, InputWriter, ParticlesInput},
};

pub struct SimulationInputImpl {
    pub directory_lock: DirectoryLock,
    pub input_header: InputHeader,
    pub input_writer: InputWriter,
    pub max_bytes_on_disk: u64,
    pub current_frame: Option<InputFrame>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct FrameStart {
    gravity: Vector3<T>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum FrameBulkMeta {
    Particles {
        object_name: String,
        captured_attribute: FrameBulkParticle,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum FrameBulkParticle {
    Transforms,
    Sizes,
    Densities,
    YoungsModuluses,
    PoissonsRatios,
    Types,
    InitialPositions,
}

pub fn simulation_input_path<P: AsRef<Path>>(cache_dir: P) -> PathBuf {
    cache_dir.as_ref().join("simulation_input.bin")
}

impl SimulationInputImpl {
    pub fn new(
        uuid: String,
        directory: PathBuf,
        input_header: InputHeader,
        max_bytes_on_disk: u64,
    ) -> Result<Self> {
        let directory_lock = DirectoryLock::new(directory.clone(), uuid)?;

        let input_writer = InputWriter::new(simulation_input_path(directory), &input_header)?;

        Ok(Self {
            directory_lock,
            input_header,
            input_writer,
            max_bytes_on_disk,
            current_frame: None,
        })
    }
}

impl SimulationInput for SimulationInputImpl {
    fn start_frame(&mut self, frame_start: Value) -> Result<()> {
        ensure!(self.current_frame.is_none(), "Not finished prior frame.");

        let FrameStart { gravity } = from_value(frame_start)?;

        let input_frame = InputFrame {
            gravity,
            particles_input: self
                .input_header
                .objects
                .iter()
                .filter_map(|input_object| {
                    matches!(input_object.ty, InputObjectType::Particles)
                        .then_some((input_object.name.clone(), ParticlesInput::default()))
                })
                .collect(),
        };
        info!("starting next frame: {input_frame:?}");

        self.current_frame = Some(input_frame);

        Ok(())
    }

    fn record_input(&mut self, meta: Value, bulk: InputBulk) -> Result<()> {
        let Some(current_frame) = self.current_frame.as_mut() else {
            bail!("No frame started.");
        };
        info!("got some input: {meta:?}");
        match from_value::<FrameBulkMeta>(meta)? {
            FrameBulkMeta::Particles {
                object_name,
                captured_attribute,
            } => {
                let ps = current_frame
                    .particles_input
                    .get_mut(&object_name)
                    .context("Missing input particle object")?;
                match captured_attribute {
                    FrameBulkParticle::Transforms => ps.transforms = bulk.try_into()?,
                    FrameBulkParticle::Sizes => ps.sizes = bulk.try_into()?,
                    FrameBulkParticle::Densities => ps.densities = bulk.try_into()?,
                    FrameBulkParticle::YoungsModuluses => ps.youngs_moduluses = bulk.try_into()?,
                    FrameBulkParticle::PoissonsRatios => ps.poissons_ratios = bulk.try_into()?,
                    FrameBulkParticle::Types => ps.types = bulk.try_into()?,
                    FrameBulkParticle::InitialPositions => ps.types = bulk.try_into()?,
                }
            }
        }
        Ok(())
    }

    fn finish_frame(&mut self) -> Result<()> {
        let Some(current_frame) = self.current_frame.take() else {
            bail!("No frame started.");
        };

        self.input_writer.record_frame(current_frame)?;

        ensure!(
            self.input_writer.size()? < self.max_bytes_on_disk,
            "Exceeding allowed disk space."
        );

        Ok(())
    }
}
