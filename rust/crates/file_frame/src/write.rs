// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::BTreeSet;

use squishy_volumes_util::attribute;
use strum::IntoEnumIterator as _;

use crate::*;

impl Frame {
    pub fn write(self, path: impl AsRef<std::path::Path>) -> Result<u64, Error> {
        let temp = Self::get_temp(&path).map_err(|error| Error::Write {
            path: path.as_ref().to_path_buf(),
            error,
        })?;

        let size = self.write_temp(&temp).map_err(|error| Error::Write {
            path: temp.clone(),
            error,
        })?;

        std::fs::rename(&temp, &path).map_err(|error| Error::Write {
            path: path.as_ref().to_path_buf(),
            error: error.into(),
        })?;

        Ok(size)
    }

    fn get_temp(path: impl AsRef<std::path::Path>) -> Result<std::path::PathBuf, WriteError> {
        let Some(dir) = path.as_ref().parent() else {
            return Err(WriteError::NoParent);
        };
        Ok(dir.join("temp.bin"))
    }

    fn write_temp(mut self, path: impl AsRef<std::path::Path>) -> Result<u64, WriteError> {
        let Some(dir) = path.as_ref().parent() else {
            return Err(WriteError::NoParent);
        };
        let temp = dir.join("temp.bin");

        let mut writer = alembic_rust::ArchiveWriter::create(std::io::BufWriter::new(
            std::fs::File::create(&temp)?,
        ))?;

        writer
            .metadata_mut()
            .set(APPLICATION_KEY, SQUISHY_VOLUMES)?;

        for (name, particles) in &mut self.io_state.particle_objects {
            write_points(
                writer.begin_points(name, particles.num_particles)?,
                particles,
            )
            .map_err(|error| WriteError::Points {
                name: name.clone(),
                error,
            })?;
        }

        writer.user_data_with(SQUISHY_VOLUMES_BLOB, |w| {
            squishy_volumes_file_util::write_magic_and_version(magic_bytes, w)?;
            bincode::serialize_into(w, &self)?;
            Ok::<_, WriteError>(())
        })??;

        let writer = writer.finish()?;

        let written_bytes = writer
            .into_inner()
            .map_err(WriteError::Flush)?
            .metadata()?
            .len();

        Ok(written_bytes)
    }
}

fn write_points<'a, W: Write + Seek>(
    mut writer: alembic_rust::PointsWriter<'a, W>,
    particles: &mut ParticlesObject,
) -> Result<(), WriteErrorPoints> {
    let missing = attribute::Key::iter()
        .filter(|key| !particles.attributes.contains_key(key))
        .collect::<BTreeSet<_>>();
    if !missing.is_empty() {
        return Err(WriteErrorPoints::Incomplete(missing));
    }

    for key in &particles.alembic_attributes {
        (|| {
            let value = particles
                .attributes
                .remove(key)
                .ok_or(WriteErrorPointsAttribute::Missing)?;
            match value {
                AttributeValue::Float1(items) => {
                    writer.attribute(key.alembic_name(), &items)?;
                }
                AttributeValue::Float2(items) => {
                    writer.attribute(key.alembic_name(), &items)?;
                }
                AttributeValue::Float3(items) => {
                    writer.attribute(key.alembic_name(), &items)?;
                }
            }
            Ok(())
        })()
        .map_err(|error| WriteErrorPoints::Attribute {
            name: key.alembic_name().to_string(),
            error,
        })?;
    }
    writer.finish()?;
    Ok(())
}
