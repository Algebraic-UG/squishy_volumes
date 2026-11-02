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

use alembic_rust::AttributeType;
use squishy_volumes_util::attribute;
use strum::IntoEnumIterator as _;

use crate::*;

impl Frame {
    pub fn read(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        Self::read_inner(&path).map_err(|error| Error::Read {
            path: path.as_ref().to_path_buf(),
            error,
        })
    }

    fn read_inner(path: impl AsRef<std::path::Path>) -> Result<Self, ReadError> {
        let (content, mut reader) =
            alembic_rust::read_archive(std::io::BufReader::new(std::fs::File::open(&path)?))?;

        let application_key = content.metadata.get(APPLICATION_KEY);
        if Some(SQUISHY_VOLUMES) != application_key {
            return Err(ReadError::ApplicationKey(
                application_key.map(str::to_string),
            ));
        }

        let Some(user_data) = content.user_data.get(SQUISHY_VOLUMES_BLOB) else {
            return Err(ReadError::MissingBlob);
        };

        let mut blob_reader = reader.user_data_reader(user_data)?;
        squishy_volumes_file_util::read_magic_and_version(magic_bytes, &mut blob_reader)?;
        let mut frame: Self = bincode::deserialize_from(&mut blob_reader)?;
        drop(blob_reader);

        for (name, particles) in &mut frame.io_state.particle_objects {
            read_points(
                content
                    .points
                    .get(name)
                    .ok_or(ReadError::MissingPoints(name.clone()))?,
                &mut reader,
                particles,
            )
            .map_err(|error| ReadError::Points {
                name: name.clone(),
                error,
            })?;
        }

        Ok(frame)
    }
}

fn read_points<R: Read + Seek>(
    points: &Points,
    reader: &mut ArchiveReader<R>,
    particles: &mut ParticlesObject,
) -> Result<(), ReadErrorPoints> {
    for key in &particles.alembic_attributes {
        (|| -> Result<(), ReadErrorPointsAttribute> {
            let handle = points
                .attributes
                .get(key.alembic_name())
                .ok_or(ReadErrorPointsAttribute::Missing)?;

            let value = match key.alembic_type() {
                AttributeType::Float1 => {
                    let mut data = vec![Default::default(); particles.num_particles];
                    reader.read_into(&handle.typed()?, &mut data)?;
                    AttributeValue::Float1(data)
                }
                AttributeType::Float2 => {
                    let mut data = vec![Default::default(); particles.num_particles];
                    reader.read_into(&handle.typed()?, &mut data)?;
                    AttributeValue::Float2(data)
                }
                AttributeType::Float3 => {
                    let mut data = vec![Default::default(); particles.num_particles];
                    reader.read_into(&handle.typed()?, &mut data)?;
                    AttributeValue::Float3(data)
                }
            };

            if particles.attributes.insert(*key, value).is_some() {
                return Err(ReadErrorPointsAttribute::Duplicate);
            }

            Ok(())
        })()
        .map_err(|error| ReadErrorPoints::Attribute {
            name: key.alembic_name().to_string(),
            error,
        })?;
    }

    let missing = attribute::Key::iter()
        .filter(|key| !particles.attributes.contains_key(key))
        .collect::<BTreeSet<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(ReadErrorPoints::Incomplete(missing))
    }
}
