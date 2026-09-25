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
use squishy_volumes_file_input::{FrameBulk, FrameBulkMeta, InputHeader, InputWriter};
use squishy_volumes_util::AnimatedGlobals;

use crate::Error;

pub struct SimulationInputImpl {
    pub directory_lock: DirectoryLock,
    pub input_writer: InputWriter,
    pub max_bytes_on_disk: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FrameStart {
    pub animated_globals: AnimatedGlobals,
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
        })
    }

    pub fn clean_up(self) {
        drop(self.input_writer);
        if let Err(e) = remove_file(simulation_input_path(self.directory_lock.directory())) {
            tracing::error!("failed to clean up input file: {e:?}");
        }
    }
}

impl SimulationInputImpl {
    pub fn start_frame_impl(&mut self, frame_start: Value) -> Result<(), Error> {
        let FrameStart { animated_globals } =
            from_value(frame_start).map_err(Error::ParsingFrameStart)?;

        let size = self
            .input_writer
            .start_frame(&animated_globals)
            .map_err(Error::StartFrame)?;

        self.check_vs_max_bytes(size)
    }

    pub fn record_input_impl(&mut self, meta: Value, bulk: InputBulk) -> Result<(), Error> {
        let meta = from_value::<FrameBulkMeta>(meta).map_err(Error::ParsingBulkMeta)?;

        let size = self
            .input_writer
            .record_bulk(&FrameBulk { meta, data: bulk })
            .map_err(Error::RecordFrame)?;

        self.check_vs_max_bytes(size)
    }

    fn check_vs_max_bytes(&self, size: u64) -> Result<(), Error> {
        if size > self.max_bytes_on_disk {
            Err(Error::DiskSpaceExceededWhileRecording(
                self.max_bytes_on_disk,
            ))
        } else {
            Ok(())
        }
    }
}
