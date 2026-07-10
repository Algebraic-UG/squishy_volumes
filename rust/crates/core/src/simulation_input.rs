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
use squishy_volumes_file_frame::ParticleFlags;
use squishy_volumes_file_input::{InputFrame, InputHeader, InputWriter};
use tracing::{debug, error};

use crate::{Error, InputBulkError, InputBulkExt};

pub struct SimulationInputImpl {
    pub directory_lock: DirectoryLock,
    pub input_writer: InputWriter,
    pub max_bytes_on_disk: u64,
    pub current_frame: Option<InputFrame>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct FrameStart {
    gravity: [f32; 3],
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
        let FrameStart { gravity } = from_value(frame_start).map_err(Error::ParsingFrameStart)?;

        let input_frame = InputFrame {
            gravity,
            particles_inputs: Default::default(),
            collider_inputs: Default::default(),
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
        let FrameBulkMeta {
            object_name,
            captured_attribute,
        } = from_value::<FrameBulkMeta>(meta).map_err(Error::ParsingBulkMeta)?;
        let captured_attribute_copy = captured_attribute.clone();
        (|| -> Result<(), InputBulkError> {
            match captured_attribute {
                BulkAttribute::Particles(captured_attribute) => {
                    let ps = current_frame
                        .particles_inputs
                        .entry(object_name.clone())
                        .or_default();

                    match captured_attribute {
                        FrameBulkParticles::IsSolid
                        | FrameBulkParticles::IsFluid
                        | FrameBulkParticles::UseViscosity
                        | FrameBulkParticles::UseSandAlpha
                        | FrameBulkParticles::HasGoal => {
                            let slice = bulk.as_bools()?;
                            if ps.flags.is_empty() {
                                ps.flags.resize(slice.len(), Default::default());
                            } else {
                                if ps.flags.len() == bulk.len() {
                                    return Err(InputBulkError::FlagsLengthChanged);
                                }
                            }
                            let flag = match captured_attribute {
                                FrameBulkParticles::IsSolid => ParticleFlags::IS_SOLID,
                                FrameBulkParticles::IsFluid => ParticleFlags::IS_FLUID,
                                FrameBulkParticles::UseViscosity => ParticleFlags::USE_VISCOSITY,
                                FrameBulkParticles::UseSandAlpha => ParticleFlags::USE_SAND_ALPHA,
                                FrameBulkParticles::HasGoal => ParticleFlags::HAS_GOAL,
                                _ => unreachable!(),
                            };
                            ps.flags.iter_mut().zip(slice).for_each(|(flags, &value)| {
                                if value {
                                    *flags |= flag.bits()
                                }
                            });
                        }
                        FrameBulkParticles::Transforms => {
                            ps.transforms = bytemuck::try_cast_slice(bulk.as_floats()?)?.to_vec()
                        }
                        FrameBulkParticles::Sizes => ps.sizes = bulk.as_floats()?.to_vec(),
                        FrameBulkParticles::Densities => ps.densities = bulk.as_floats()?.to_vec(),
                        FrameBulkParticles::YoungsModuluses => {
                            ps.youngs_moduluses = bulk.as_floats()?.to_vec()
                        }
                        FrameBulkParticles::PoissonsRatios => {
                            ps.poissons_ratios = bulk.as_floats()?.to_vec()
                        }
                        FrameBulkParticles::InitialPositions => {
                            ps.initial_positions =
                                bytemuck::try_cast_slice(bulk.as_floats()?)?.to_vec()
                        }
                        FrameBulkParticles::InitialVelocity => {
                            ps.initial_velocities =
                                bytemuck::try_cast_slice(bulk.as_floats()?)?.to_vec()
                        }
                        FrameBulkParticles::ViscosityDynamic => {
                            ps.viscosities_dynamic = bulk.as_floats()?.to_vec()
                        }
                        FrameBulkParticles::ViscosityBulk => {
                            ps.viscosities_bulk = bulk.as_floats()?.to_vec()
                        }
                        FrameBulkParticles::Exponent => {
                            ps.exponents = bytemuck::try_cast_slice(bulk.as_ints()?)?.to_vec()
                        }
                        FrameBulkParticles::BulkModulus => {
                            ps.bulk_moduluses = bulk.as_floats()?.to_vec()
                        }
                        FrameBulkParticles::SandAlpha => {
                            ps.sand_alphas = bulk.as_floats()?.to_vec()
                        }
                        FrameBulkParticles::GoalPositions => {
                            ps.goal_positions =
                                bytemuck::try_cast_slice(bulk.as_floats()?)?.to_vec()
                        }
                    }
                }
                BulkAttribute::Collider(captured_attribute) => {
                    let cs = current_frame
                        .collider_inputs
                        .entry(object_name.clone())
                        .or_default();
                    match captured_attribute {
                        FrameBulkCollider::VertexPositions => {
                            cs.vertex_positions =
                                bytemuck::try_cast_slice(bulk.as_floats()?)?.to_vec()
                        }
                        FrameBulkCollider::Triangles => {
                            cs.triangle_indices =
                                bytemuck::try_cast_slice(bulk.as_ints()?)?.to_vec()
                        }
                        FrameBulkCollider::TriangleFrictions => {
                            cs.triangle_frictions = bulk.as_floats()?.to_vec()
                        }
                    }
                }
            }
            Ok(())
        })()
        .map_err(|error| Error::InputBulkError {
            object_name,
            attribute: format!("{captured_attribute_copy:?}"),
            error,
        })
    }

    pub fn finish_frame(&mut self) -> Result<(), Error> {
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
