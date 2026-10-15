// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use crate::*;

impl Frame {
    pub fn read(
        ranges: &squishy_volumes_file_input::InputRanges,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, Error> {
        Self::read_inner(ranges, &path).map_err(|error| Error::Read {
            path: path.as_ref().to_path_buf(),
            error,
        })
    }

    fn read_inner(
        ranges: &squishy_volumes_file_input::InputRanges,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, ReadError> {
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

        let mut blob_reader = reader.user_data_reader(&user_data)?;
        squishy_volumes_file_util::read_magic_and_version(magic_bytes, &mut blob_reader)?;
        let mut frame: Self = bincode::deserialize_from(&mut blob_reader)?;
        drop(blob_reader);

        // TODO: also get gridnodes, maybe bvh
        let Particles {
            elastic_energies,
            positions,
            velocities,
            initial_positions,

            flags: flags_u32,
            parameters: parameters_combined,
            collider_bits: collider_bits_u32,
            position_gradients: position_gradients_combined,
            velocity_gradients: velocity_gradients_combined,
        } = &mut frame.io_state.particles;

        let mut split_parameters =
            squishy_volumes_util::SplitParameters::with_capacity(ranges.total_particles);

        let mut flags = Vec::<[f32; 2]>::with_capacity(ranges.total_particles);
        let mut collider_bits = Vec::<[f32; 2]>::with_capacity(ranges.total_particles);
        let mut position_gradients_x = Vec::<[f32; 3]>::with_capacity(ranges.total_particles);
        let mut position_gradients_y = Vec::<[f32; 3]>::with_capacity(ranges.total_particles);
        let mut position_gradients_z = Vec::<[f32; 3]>::with_capacity(ranges.total_particles);
        let mut velocity_gradients_x = Vec::<[f32; 3]>::with_capacity(ranges.total_particles);
        let mut velocity_gradients_y = Vec::<[f32; 3]>::with_capacity(ranges.total_particles);
        let mut velocity_gradients_z = Vec::<[f32; 3]>::with_capacity(ranges.total_particles);

        let mut alembic_data = AlembicData {
            elastic_energies,
            positions,
            velocities,
            initial_positions,
            flags: &mut flags,
            split_parameters: &mut split_parameters,
            collider_bits: &mut collider_bits,
            position_gradients_x: &mut position_gradients_x,
            position_gradients_y: &mut position_gradients_y,
            position_gradients_z: &mut position_gradients_z,
            velocity_gradients_x: &mut velocity_gradients_x,
            velocity_gradients_y: &mut velocity_gradients_y,
            velocity_gradients_z: &mut velocity_gradients_z,
        };

        for (name, range) in &ranges.objects {
            let squishy_volumes_file_input::InputRange::Particles(
                squishy_volumes_file_input::InputRangeParticles { particle_range },
            ) = range
            else {
                continue;
            };
            PointsReader::read_points(
                content
                    .points
                    .get(name)
                    .ok_or(ReadError::MissingPoints(name.clone()))?,
                &mut reader,
                *particle_range,
                &mut alembic_data,
            )
            .map_err(|error| ReadError::Points {
                name: name.clone(),
                error,
            })?;
        }

        *flags_u32 = flags
            .into_iter()
            .map(two_f32_to_u32)
            .map(squishy_volumes_util::ParticleFlags)
            .collect();
        *collider_bits_u32 = collider_bits.into_iter().map(two_f32_to_u32).collect();
        *parameters_combined = split_parameters.to_zipped();
        *position_gradients_combined = combine_matrices([
            position_gradients_x,
            position_gradients_y,
            position_gradients_z,
        ]);
        *velocity_gradients_combined = combine_matrices([
            velocity_gradients_x,
            velocity_gradients_y,
            velocity_gradients_z,
        ]);

        Ok(frame)
    }
}

struct PointsReader<'a, R> {
    points: &'a Points,
    reader: &'a mut ArchiveReader<R>,
    particle_range: Range<usize>,
}

impl<'a, R: Read + Seek> PointsReader<'a, R> {
    fn r<T: AttributeElement>(
        &mut self,
        name: &str,
        destination: &mut [T],
    ) -> Result<(), ReadErrorPoints> {
        self.reader
            .read_into(
                &self
                    .points
                    .attributes
                    .get(name)
                    .ok_or(ReadErrorPoints::AttributeMissing(name.to_string()))?
                    .typed()
                    .map_err(|error| ReadErrorPoints::Alembic {
                        name: name.to_string(),
                        error,
                    })?,
                &mut destination[self.particle_range],
            )
            .map_err(|error| ReadErrorPoints::Alembic {
                name: name.to_string(),
                error,
            })
    }
    fn read_points(
        points: &'a Points,
        reader: &'a mut ArchiveReader<R>,
        particle_range: Range<usize>,
        AlembicData {
            elastic_energies,
            positions,
            velocities,
            initial_positions,
            flags,
            split_parameters:
                SplitParameters {
                    density,
                    initial_volume,
                    viscosity_dynamic,
                    viscosity_bulk,
                    youngs_modulus,
                    poissons_ratio,
                    sand_alpha,
                    bulk_modulus,
                    exponent,
                },
            collider_bits,
            position_gradients_x,
            position_gradients_y,
            position_gradients_z,
            velocity_gradients_x,
            velocity_gradients_y,
            velocity_gradients_z,
        }: &mut AlembicData,
    ) -> Result<(), ReadErrorPoints> {
        let mut r = Self {
            points,
            reader,
            particle_range,
        };

        r.r(FLAGS, flags)?;
        r.r(DENSITY, density)?;
        r.r(INITIAL_VOLUME, initial_volume)?;
        r.r(VISCOSITY_DYNAMIC, viscosity_dynamic)?;
        r.r(VISCOSITY_BULK, viscosity_bulk)?;
        r.r(YOUNGS_MODULUS, youngs_modulus)?;
        r.r(POISSONS_RATIO, poissons_ratio)?;
        r.r(SAND_ALPHA, sand_alpha)?;
        r.r(BULK_MODULUS, bulk_modulus)?;
        r.r(EXPONENT, exponent)?;
        r.r(ELASTIC_ENERGIES, elastic_energies)?;
        r.r(ELASTIC_ENERGIES, elastic_energies)?;
        r.r(COLLIDER_BITS, collider_bits)?;
        r.r(POSITIONS, positions)?;
        r.r(POSITION_GRADIENTS_X, position_gradients_x)?;
        r.r(POSITION_GRADIENTS_Y, position_gradients_y)?;
        r.r(POSITION_GRADIENTS_Z, position_gradients_z)?;
        r.r(VELOCITIES, velocities)?;
        r.r(VELOCITY_GRADIENTS_X, velocity_gradients_x)?;
        r.r(VELOCITY_GRADIENTS_Y, velocity_gradients_y)?;
        r.r(VELOCITY_GRADIENTS_Z, velocity_gradients_z)?;
        r.r(INITIAL_POSITIONS, initial_positions)?;

        Ok(())
    }
}
