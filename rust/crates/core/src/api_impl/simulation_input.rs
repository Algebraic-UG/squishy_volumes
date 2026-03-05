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

use anyhow::{Context, Result, bail, ensure};
use bitflags::bitflags;
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value};
use squishy_volumes_api::{InputBulk, SimulationInput, T};
use tracing::{debug, error};

use crate::{
    directory_lock::DirectoryLock,
    input_file::{
        ColliderInput, InputFrame, InputHeader, InputObject, InputWriter, ParticlesInput,
    },
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

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct ParticleFlags(pub i32);

bitflags! {
    impl ParticleFlags: i32 {
        const IsSolid = 1 << 0;
        const IsFluid = 1 << 1;
        const UseViscosity = 1 << 2;
        const UseSandAlpha = 1 << 3;
        const HasGoal = 1 << 4;
        const _ = !0;
    }
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

    pub fn clean_up(self) {
        drop(self.input_writer);
        if let Err(e) = remove_file(simulation_input_path(self.directory_lock.directory())) {
            error!("failed to clean up input file: {e:?}");
        }
    }
}

impl SimulationInput for SimulationInputImpl {
    fn start_frame(&mut self, frame_start: Value) -> Result<()> {
        let FrameStart { gravity } = from_value(frame_start)?;

        let input_frame = InputFrame {
            gravity,
            particles_inputs: self
                .input_header
                .objects
                .iter()
                .filter_map(|(name, input_object)| {
                    matches!(input_object, InputObject::Particles)
                        .then_some((name.clone(), ParticlesInput::default()))
                })
                .collect(),
            collider_inputs: self
                .input_header
                .objects
                .iter()
                .filter_map(|(name, input_object)| {
                    matches!(input_object, InputObject::Collider { .. })
                        .then_some((name.clone(), ColliderInput::default()))
                })
                .collect(),
        };
        debug!("starting next frame: {input_frame:?}");

        self.current_frame = Some(input_frame);

        Ok(())
    }

    fn record_input(&mut self, meta: Value, bulk: InputBulk) -> Result<()> {
        let Some(current_frame) = self.current_frame.as_mut() else {
            bail!("No frame started.");
        };
        debug!("got some input: {meta:?}");
        let FrameBulkMeta {
            object_name,
            captured_attribute,
        } = from_value::<FrameBulkMeta>(meta)?;
        match captured_attribute {
            BulkAttribute::Particles(captured_attribute) => {
                let ps = current_frame
                    .particles_inputs
                    .get_mut(&object_name)
                    .context("Missing input particle object")?;
                match captured_attribute {
                    FrameBulkParticles::IsSolid
                    | FrameBulkParticles::IsFluid
                    | FrameBulkParticles::UseViscosity
                    | FrameBulkParticles::UseSandAlpha
                    | FrameBulkParticles::HasGoal => {
                        ps.flags.resize(bulk.len(), Default::default());
                        let flag = match captured_attribute {
                            FrameBulkParticles::IsSolid => ParticleFlags::IsSolid,
                            FrameBulkParticles::IsFluid => ParticleFlags::IsFluid,
                            FrameBulkParticles::UseViscosity => ParticleFlags::UseViscosity,
                            FrameBulkParticles::UseSandAlpha => ParticleFlags::UseSandAlpha,
                            FrameBulkParticles::HasGoal => ParticleFlags::HasGoal,
                            _ => unreachable!(),
                        };
                        ps.flags
                            .iter_mut()
                            .zip(bulk.as_bools()?)
                            .for_each(|(flags, &value)| {
                                if value {
                                    *flags |= flag.clone()
                                }
                            });
                    }
                    FrameBulkParticles::Transforms => {
                        ensure!(bulk.len() % 16 == 0);
                        ps.transforms = bulk.try_into()?;
                    }
                    FrameBulkParticles::Sizes => ps.sizes = bulk.try_into()?,
                    FrameBulkParticles::Densities => ps.densities = bulk.try_into()?,
                    FrameBulkParticles::YoungsModuluses => ps.youngs_moduluses = bulk.try_into()?,
                    FrameBulkParticles::PoissonsRatios => ps.poissons_ratios = bulk.try_into()?,
                    FrameBulkParticles::InitialPositions => {
                        ensure!(bulk.len() % 3 == 0);
                        ps.initial_positions = bulk.try_into()?
                    }
                    FrameBulkParticles::InitialVelocity => {
                        ensure!(bulk.len() % 3 == 0);
                        ps.initial_velocities = bulk.try_into()?
                    }
                    FrameBulkParticles::ViscosityDynamic => {
                        ps.viscosities_dynamic = bulk.try_into()?
                    }
                    FrameBulkParticles::ViscosityBulk => ps.viscosities_bulk = bulk.try_into()?,
                    FrameBulkParticles::Exponent => ps.exponents = bulk.try_into()?,
                    FrameBulkParticles::BulkModulus => ps.bulk_moduluses = bulk.try_into()?,
                    FrameBulkParticles::SandAlpha => ps.sand_alphas = bulk.try_into()?,
                    FrameBulkParticles::GoalPositions => ps.goal_positions = bulk.try_into()?,
                }
            }
            BulkAttribute::Collider(captured_attribute) => {
                let cs = current_frame
                    .collider_inputs
                    .get_mut(&object_name)
                    .context("Missing input collider object")?;
                match captured_attribute {
                    FrameBulkCollider::VertexPositions => cs.vertex_positions = bulk.try_into()?,
                    FrameBulkCollider::Triangles => cs.triangles = bulk.try_into()?,
                    FrameBulkCollider::TriangleFrictions => {
                        cs.triangle_frictions = bulk.try_into()?
                    }
                }
            }
        }
        Ok(())
    }

    fn finish_frame(&mut self) -> Result<()> {
        let Some(current_frame) = self.current_frame.take() else {
            bail!("No frame started.");
        };

        for ParticlesInput {
            // these have to be complete
            flags,
            transforms,
            sizes,
            densities,
            initial_positions,

            youngs_moduluses: _,
            poissons_ratios: _,
            initial_velocities: _,
            viscosities_dynamic: _,
            viscosities_bulk: _,
            exponents: _,
            bulk_moduluses: _,
            sand_alphas: _,
            goal_positions: _,
            goal_stiffnesses: _,
        } in current_frame.particles_inputs.values()
        {
            let n = flags.len();
            ensure!(n == transforms.len() / 16);
            ensure!(n == sizes.len());
            ensure!(n == densities.len());
            ensure!(n == initial_positions.len() / 3);
        }

        for (
            name,
            ColliderInput {
                // these have to be complete
                vertex_positions,
                triangles,

                triangle_frictions: _,
            },
        ) in current_frame.collider_inputs.iter()
        {
            let InputObject::Collider { num_vertices } = self
                .input_header
                .objects
                .get(name)
                .context("Missing collider object")?
            else {
                bail!("Input object type changed");
            };
            ensure!(*num_vertices == vertex_positions.len() / 3);
            ensure!(triangles.iter().all(|&vertex_idx| {
                vertex_idx >= 0 && (vertex_idx as usize) < vertex_positions.len()
            }));
        }

        self.input_writer.record_frame(&current_frame)?;

        ensure!(
            self.input_writer.size()? < self.max_bytes_on_disk,
            "Exceeding allowed disk space."
        );

        Ok(())
    }
}
