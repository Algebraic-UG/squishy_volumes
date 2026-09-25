// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    fs::remove_file,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value};
use squishy_volumes_api::InputBulk;
use squishy_volumes_directory_lock::DirectoryLock;
use squishy_volumes_file_input::{InputFrame, InputHeader, InputWriter};
use squishy_volumes_util::{AnimatedGlobals, ParticleFlags};
use tracing::{debug, error};

use crate::{Error, InputBulkError, InputBulkExt};

pub struct SimulationInputImpl<'a> {
    pub directory_lock: DirectoryLock,
    pub input_writer: InputWriter,
    pub max_bytes_on_disk: u64,
    pub current_frame: Option<InputFrame<'a>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FrameStart {
    pub animated_globals: AnimatedGlobals,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct FrameBulkMeta {
    object_name: String,
    captured_attribute: BulkAttribute,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum BulkAttribute {
    Particles(FrameBulkParticles),
    Collider(FrameBulkCollider),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum FrameBulkParticles {
    IsSolid,
    IsFluid,
    UseViscosity,
    UseSandAlpha,
    HasGoal,
    Transforms,
    Sizes,
    Densities,
    YoungsModuluses,
    PoissonsRatios,
    InitialPositions,
    InitialVelocity,
    ViscosityDynamic,
    ViscosityBulk,
    Exponent,
    BulkModulus,
    SandAlpha,
    GoalPositions,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum FrameBulkCollider {
    VertexPositions,
    Triangles,
    TriangleFrictions,
    TriangleDampings,
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
    ) -> Result<Self, Error> {
        let directory_lock = DirectoryLock::new(directory.clone(), uuid)?;

        let input_writer = InputWriter::new(simulation_input_path(directory), input_header)
            .map_err(Error::StartInputWriting)?;

        Ok(Self {
            directory_lock,
            input_writer,
            max_bytes_on_disk,
            current_frame: None,
        })
    }

    pub fn clean_up(self) {
        drop(self.input_writer);
        if let Err(e) = remove_file(simulation_input_path(self.directory_lock.directory())) {
            error!("failed to clean up input file: {e:?}");
        }
    }
}

impl SimulationInputImpl {
    pub fn start_frame_impl(&mut self, frame_start: Value) -> Result<(), Error> {
        let FrameStart { animated_globals } =
            from_value(frame_start).map_err(Error::ParsingFrameStart)?;

        let input_frame = InputFrame {
            animated_globals,
            bulk: Default::default(),
        };
        debug!("starting next frame: {input_frame:?}");

        self.current_frame = Some(input_frame);

        Ok(())
    }

    pub fn record_input_impl(&mut self, meta: Value, bulk: InputBulk) -> Result<(), Error> {
        let Some(current_frame) = self.current_frame.as_mut() else {
            return Err(Error::NoFrameStarted);
        };
        debug!("got some input: {meta:?}");
        let meta = from_value::<FrameBulkMeta>(meta).map_err(Error::ParsingBulkMeta)?;
        let data = 
    }

    pub fn finish_frame_impl(&mut self) -> Result<(), Error> {
        let Some(current_frame) = self.current_frame.take() else {
            return Err(Error::NoFrameStarted);
        };
        self.input_writer
            .record_frame(&current_frame)
            .map_err(Error::RecordFrame)?;

        if self.input_writer.size().map_err(Error::QuerySize)? > self.max_bytes_on_disk {
            return Err(Error::DiskSpaceExceededWhileRecording(
                self.max_bytes_on_disk,
            ));
        }

        Ok(())
        }
}
