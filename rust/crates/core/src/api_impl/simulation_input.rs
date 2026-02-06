// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use bitflags::bitflags;
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
    Flags,
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
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct ParticleFlags(pub i32);

bitflags! {
    impl ParticleFlags: i32 {
        const IsSolid = 1 << 0;
        const IsFluid = 1 << 1;
        const UseViscosity = 1 << 2;
        const UseSandAlpha = 1 << 3;
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
                    FrameBulkParticle::Flags => ps.flags = bulk.try_into()?,
                    FrameBulkParticle::Transforms => {
                        ensure!(bulk.len() % 16 == 0);
                        ps.transforms = bulk.try_into()?;
                    }
                    FrameBulkParticle::Sizes => ps.sizes = bulk.try_into()?,
                    FrameBulkParticle::Densities => ps.densities = bulk.try_into()?,
                    FrameBulkParticle::YoungsModuluses => ps.youngs_moduluses = bulk.try_into()?,
                    FrameBulkParticle::PoissonsRatios => ps.poissons_ratios = bulk.try_into()?,
                    FrameBulkParticle::InitialPositions => {
                        ensure!(bulk.len() % 3 == 0);
                        ps.initial_positions = bulk.try_into()?
                    }
                    FrameBulkParticle::InitialVelocity => {
                        ensure!(bulk.len() % 3 == 0);
                        ps.initial_velocity = bulk.try_into()?
                    }
                    FrameBulkParticle::ViscosityDynamic => {
                        ps.viscosity_dynamic = bulk.try_into()?
                    }
                    FrameBulkParticle::ViscosityBulk => ps.viscosity_bulk = bulk.try_into()?,
                    FrameBulkParticle::Exponent => ps.exponent = bulk.try_into()?,
                    FrameBulkParticle::BulkModulus => ps.bulk_modulus = bulk.try_into()?,
                    FrameBulkParticle::SandAlpha => ps.sand_alpha = bulk.try_into()?,
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
            flags,
            transforms,
            sizes,
            densities,
            youngs_moduluses,
            poissons_ratios,
            initial_positions,
            initial_velocity,
            viscosity_dynamic,
            viscosity_bulk,
            exponent,
            bulk_modulus,
            sand_alpha,
        } in current_frame.particles_input.values()
        {
            let n = flags.len();
            ensure!(n == transforms.len() / 16);
            ensure!(n == sizes.len());
            ensure!(n == densities.len());
            ensure!(n == youngs_moduluses.len());
            ensure!(n == poissons_ratios.len());
            ensure!(n == initial_positions.len() / 3);
            ensure!(n == initial_velocity.len() / 3);
            ensure!(n == viscosity_dynamic.len());
            ensure!(n == viscosity_bulk.len());
            ensure!(n == exponent.len());
            ensure!(n == bulk_modulus.len());
            ensure!(n == sand_alpha.len());
        }

        self.input_writer.record_frame(current_frame)?;

        ensure!(
            self.input_writer.size()? < self.max_bytes_on_disk,
            "Exceeding allowed disk space."
        );

        Ok(())
    }
}
