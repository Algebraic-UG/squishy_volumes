// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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
