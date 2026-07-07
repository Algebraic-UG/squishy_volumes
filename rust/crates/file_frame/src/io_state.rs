// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

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
