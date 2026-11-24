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
        let temp = Self::get_temp(&path).map_err(|source| Error::Write {
            path: path.as_ref().to_path_buf(),
            source,
        })?;

        let size = self.write_temp(&temp).map_err(|source| Error::Write {
            path: temp.clone(),
            source,
        })?;

        std::fs::rename(&temp, &path).map_err(|source| Error::Write {
            path: path.as_ref().to_path_buf(),
            source: WriteError::Rename { from: temp, source },
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
        let file = std::fs::File::create(path).map_err(|source| WriteError::Create { source })?;
        let mut writer = alembic_rust::ArchiveWriter::create(std::io::BufWriter::new(file))
            .map_err(|source| WriteError::Archive { source })?;

        writer
            .metadata_mut()
            .set(APPLICATION_KEY, SQUISHY_VOLUMES)
            .map_err(|source| WriteError::ApplicationKey { source })?;

        for (name, particles) in &mut self.io_state.particle_objects {
            writer
                .begin_points(name, particles.num_particles)
                .map_err(|source| WritePointsError::Begin { source })
                .and_then(|points_writer| write_points(points_writer, particles))
                .map_err(|source| WriteError::Points {
                    name: name.clone(),
                    source,
                })?;
        }

        writer
            .user_data_with(SQUISHY_VOLUMES_BLOB, |w| {
                squishy_volumes_file_util::write_magic_and_version(magic_bytes, w)?;
                bincode::serialize_into(w, &self)?;
                Ok::<_, WriteError>(())
            })
            .map_err(|source| WriteError::Blob { source })??;

        let writer = writer
            .finish()
            .map_err(|source| WriteError::Finish { source })?;

        let written_bytes = writer
            .into_inner()
            .map_err(|source| WriteError::Flush { source })?
            .metadata()
            .map_err(|source| WriteError::Size { source })?
            .len();

        Ok(written_bytes)
    }
}

fn write_points<'a, W: Write + Seek>(
    mut writer: alembic_rust::PointsWriter<'a, W>,
    particles: &mut ParticlesObject,
) -> Result<(), WritePointsError> {
    let missing = attribute::Key::iter()
        .filter(|key| !particles.attributes.contains_key(key))
        .collect::<BTreeSet<_>>();
    if !missing.is_empty() {
        return Err(WritePointsError::Incomplete(missing));
    }

    for key in &particles.alembic_attributes {
        (|| {
            let value = particles
                .attributes
                .remove(key)
                .ok_or(WriteAttributeError::Missing)?;
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
        .map_err(|source| WritePointsError::Attribute {
            name: key.alembic_name().to_string(),
            source,
        })?;
    }
    writer
        .finish()
        .map_err(|source| WritePointsError::Finish { source })
}
