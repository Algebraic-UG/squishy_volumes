// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use crate::*;

impl Frame {
    pub fn write(
        self,
        ranges: &squishy_volumes_file_input::InputRanges,
        path: impl AsRef<std::path::Path>,
    ) -> Result<u64, Error> {
        let temp = Self::get_temp(&path).map_err(|error| Error::Write {
            path: path.as_ref().to_path_buf(),
            error,
        })?;

        let size = self
            .write_temp(ranges, &temp)
            .map_err(|error| Error::Write {
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

    fn write_temp(
        mut self,
        ranges: &squishy_volumes_file_input::InputRanges,
        path: impl AsRef<std::path::Path>,
    ) -> Result<u64, WriteError> {
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

        // TODO: also take gridnodes, maybe bvh
        let mut particles = std::mem::take(&mut self.io_state.particles);
        let Particles {
            flags,
            parameters,
            elastic_energies,
            collider_bits,
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
            initial_positions,
        } = &mut particles;

        let mut split_parameters = parameters.as_slice().into();

        // TODO: once blender can read ints we can change this
        let mut flags: Vec<[f32; 2]> = flags.iter().map(|flags| u32_to_two_f32(&flags.0)).collect();
        let mut collider_bits: Vec<[f32; 2]> = collider_bits.iter().map(u32_to_two_f32).collect();

        // unlikely this can change
        let [
            mut position_gradients_x,
            mut position_gradients_y,
            mut position_gradients_z,
        ] = split_matrices(&position_gradients);
        let [
            mut velocity_gradients_x,
            mut velocity_gradients_y,
            mut velocity_gradients_z,
        ] = split_matrices(&velocity_gradients);

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
            PointsWriter::write_points(
                writer.begin_points(&name, particle_range.end - particle_range.start)?,
                *particle_range,
                &mut alembic_data,
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

struct PointsWriter<'a, W: Write> {
    writer: alembic_rust::PointsWriter<'a, W>,
    particle_range: Range<usize>,
}
impl<'a, W: Write + Seek> PointsWriter<'a, W> {
    fn w<T: AttributeElement>(&mut self, name: &str, source: &[T]) -> Result<(), WriteErrorPoints> {
        self.writer
            .attribute(name, &source[self.particle_range])
            .map_err(|error| WriteErrorPoints::Attribute {
                name: name.to_string(),
                error,
            })
    }
    fn write_points(
        writer: alembic_rust::PointsWriter<'a, W>,
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
    ) -> Result<(), WriteErrorPoints> {
        let mut w = Self {
            writer,
            particle_range,
        };

        w.w(FLAGS, flags)?;
        w.w(DENSITY, density)?;
        w.w(INITIAL_VOLUME, initial_volume)?;
        w.w(VISCOSITY_DYNAMIC, viscosity_dynamic)?;
        w.w(VISCOSITY_BULK, viscosity_bulk)?;
        w.w(YOUNGS_MODULUS, youngs_modulus)?;
        w.w(POISSONS_RATIO, poissons_ratio)?;
        w.w(SAND_ALPHA, sand_alpha)?;
        w.w(BULK_MODULUS, bulk_modulus)?;
        w.w(EXPONENT, exponent)?;
        w.w(ELASTIC_ENERGIES, elastic_energies)?;
        w.w(COLLIDER_BITS, collider_bits)?;
        w.w(POSITIONS, positions)?;
        w.w(POSITION_GRADIENTS_X, position_gradients_x)?;
        w.w(POSITION_GRADIENTS_Y, position_gradients_y)?;
        w.w(POSITION_GRADIENTS_Z, position_gradients_z)?;
        w.w(VELOCITIES, velocities)?;
        w.w(VELOCITY_GRADIENTS_X, velocity_gradients_x)?;
        w.w(VELOCITY_GRADIENTS_Y, velocity_gradients_y)?;
        w.w(VELOCITY_GRADIENTS_Z, velocity_gradients_z)?;
        w.w(INITIAL_POSITIONS, initial_positions)?;

        w.writer.finish()?;

        Ok(())
    }
}
