// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

mod grid_nodes;
mod io;
mod particles;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Object {
    Particles { indices: Vec<u32> },
    Collider { index: u32 },
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct IoState {
    pub time: f64,

    pub objects: std::collections::BTreeMap<String, Object>,

    pub particles: Particles,

    pub grid: Option<GridNodes>,

    pub user_data: Vec<u8>,
}

#[repr(C)]
#[derive(
    Clone,
    Copy,
    bytemuck::Zeroable,
    bytemuck::Pod,
    Debug,
    PartialEq,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ParticleFlags(u32);

bitflags::bitflags! {
    impl ParticleFlags: u32{
        const IS_SOLID = 1 << 0;
        const IS_FLUID = 1 << 1;
        const USE_VISCOSITY = 1 << 2;
        const USE_SAND_ALPHA = 1 << 3;
        const HAS_GOAL = 1 << 4;
        const TOMBSTONED = 1 << 5;
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ViscosityParameters {
    pub dynamic: f32,
    pub bulk: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParticleParameters {
    pub mass: f32,
    pub initial_volume: f32,
    pub viscosity: Option<ViscosityParameters>,
    pub specific: SpecificParticleParameters,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SpecificParticleParameters {
    Solid {
        mu: f32,
        lambda: f32,
        sand_alpha: Option<f32>,
    },
    Fluid {
        exponent: i32,
        bulk_modulus: f32,
    },
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Particles {
    pub flags: Vec<ParticleFlags>,

    pub parameters: Vec<ParticleParameters>,

    pub collider_bits: Vec<u32>,

    pub positions: Vec<[f32; 3]>,
    pub position_gradients: Vec<[f32; 9]>,

    pub velocities: Vec<[f32; 3]>,
    pub velocity_gradients: Vec<[f32; 9]>,

    pub initial_positions: Vec<[f32; 3]>,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GridNodes {
    pub node_ids: Vec<[i32; 3]>,
    pub collider_bits: Vec<u32>,
    pub masses: Vec<f32>,
    pub velocites: Vec<[f32; 3]>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to determine directory of '{0}'")]
    NoParent(std::path::PathBuf),
    #[error("Failed to create '{temp}': {error}")]
    Create {
        temp: std::path::PathBuf,
        error: std::io::Error,
    },
    #[error("Failed to open '{path}': {error}")]
    Open {
        path: std::path::PathBuf,
        error: std::io::Error,
    },
    #[error("Failed to flush '{temp}': {error}")]
    Write {
        temp: std::path::PathBuf,
        error: std::io::IntoInnerError<std::io::BufWriter<std::fs::File>>,
    },
    #[error("Failed to read metadata of '{temp}': {error}")]
    Metadata {
        temp: std::path::PathBuf,
        error: std::io::Error,
    },
    #[error("Failed to move '{temp}' to '{path}': {error}")]
    Move {
        temp: std::path::PathBuf,
        path: std::path::PathBuf,
        error: std::io::Error,
    },
    #[error("Failed to serialize state: {0}")]
    Serialize(bincode::Error),
    #[error("Failed to read '{path}': {error}")]
    Read {
        path: std::path::PathBuf,
        error: std::io::Error,
    },
    #[error("Failed to serialize state: {0}")]
    Deserialize(bincode::Error),
    #[error("A simple check failed: {0}")]
    FileUtil(#[from] squishy_volumes_file_util::Error),
}

fn magic_bytes() -> [u8; squishy_volumes_file_util::MAGIC_LEN] {
    const MAGIC: [char; squishy_volumes_file_util::MAGIC_LEN] = [
        'S', 'q', 'u', 'i', 's', 'h', 'y', ' ', //
        'V', 'o', 'l', 'u', 'm', 'e', 's', ' ', //
        'F', 'r', 'a', 'm', 'e', ' ', //
        'F', 'i', 'l', 'e', ' ', //
        'M', 'a', 'g', 'i', 'c',
    ];
    std::array::from_fn(|i| MAGIC[i] as u8)
}

impl IoState {
    pub fn write(&self, path: impl AsRef<std::path::Path>) -> Result<u64, Error> {
        let Some(dir) = path.as_ref().parent() else {
            return Err(Error::NoParent(path.as_ref().to_path_buf()));
        };
        let temp = dir.join("temp.bin");
        let mut writer =
            std::io::BufWriter::new(std::fs::File::create(&temp).map_err(|error| {
                Error::Create {
                    temp: temp.clone(),
                    error,
                }
            })?);
        squishy_volumes_file_util::write_magic_and_version(magic_bytes, &mut writer)?;
        bincode::serialize_into(&mut writer, self).map_err(Error::Serialize)?;
        let written_bytes = writer
            .into_inner()
            .map_err(|error| Error::Write {
                temp: temp.clone(),
                error,
            })?
            .metadata()
            .map_err(|error| Error::Metadata {
                temp: temp.clone(),
                error,
            })?
            .len();
        std::fs::rename(&temp, &path).map_err(|error| Error::Move {
            temp,
            path: path.as_ref().to_path_buf(),
            error,
        })?;

        Ok(written_bytes)
    }

    pub fn read(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let mut reader =
            std::io::BufReader::new(std::fs::File::open(&path).map_err(|error| Error::Open {
                path: path.as_ref().to_path_buf(),
                error,
            })?);
        squishy_volumes_file_util::read_magic_and_version(magic_bytes, &mut reader)?;
        bincode::deserialize_from(&mut reader).map_err(Error::Deserialize)
    }
}
