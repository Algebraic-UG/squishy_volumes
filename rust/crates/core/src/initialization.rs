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

use std::{collections::BTreeMap, iter::repeat_n};

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use squishy_volumes_file_frame::{
    IoState, IoStateError, ParticleObjectError, ParticlesObject, Value, two_f32_to_u32,
    u32_to_two_f32,
};
use squishy_volumes_file_input::{
    BulkAttribute, FrameBulkCollider, FrameBulkParticles, FrameVerificationError, InputReader,
    Object, ObjectCollider, OwnedFrameBulk,
};
use squishy_volumes_util::{ParticleFlags, attribute};
use squishy_volumes_xpu::Harness;
use strum::IntoEnumIterator as _;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InitializationError {
    #[error("failed to set up the initial particles")]
    IoState(#[from] IoStateError),

    #[error("failed to start a progress report")]
    Harness(#[from] squishy_volumes_xpu::HarnessError),

    #[error(transparent)]
    Canceled(#[from] squishy_volumes_xpu::Canceled),

    #[error("input particle #{particle_index} of '{name}' is invalid")]
    InvalidParticle {
        name: String,
        particle_index: usize,
        source: InvalidParticle,
    },
    #[error("expected {expected} values for {name}, but found {actual}")]
    ValueCount {
        name: &'static str,
        actual: usize,
        expected: usize,
    },

    #[error("failed to read the first input frame")]
    InputFrame(#[from] squishy_volumes_file_input::Error),

    #[error("particle object '{name}' has no particles")]
    EmptyParticleObject { name: String },
}

impl From<FrameVerificationError> for InitializationError {
    fn from(error: FrameVerificationError) -> Self {
        error.attach_frame(0).into()
    }
}

#[derive(Error, Debug, Clone, Copy)]
pub enum InvalidParticle {
    #[error("this setup requires some input for {attribute}")]
    MissingInput { attribute: &'static str },

    #[error("some flags are set that are not known: {0:b}")]
    UnknownFlags(u32),

    #[error("the particle solid or fluid flag must be set, but not both")]
    SolidXorFluid,

    #[error("invalid material parameters")]
    Energy(#[from] squishy_volumes_util::EnergyError),
}

pub fn initialize_io_state(
    harness: &Harness,
    input_reader: &mut InputReader,
) -> Result<IoState, InitializationError> {
    reject_empty_particle_objects(input_reader)?;

    let input_frame = {
        let _scope = harness.scope("Input reading".to_string(), 1.try_into().unwrap())?;
        input_reader
            .read_frame(0)
            .map_err(InitializationError::InputFrame)?
    };
    let animated_globals = input_frame.animated_globals;

    harness.check()?;
    let scope = harness.scope("Allocating Objects".to_string(), 1.try_into().unwrap())?;

    let particle_objects: BTreeMap<String, ParticlesObject> = input_reader
        .header()
        .objects
        .iter()
        .filter_map(|(name, object)| {
            if let Object::Particles(object) = object {
                Some((name, object))
            } else {
                None
            }
        })
        .map(|(name, object)| {
            let n = object.particle_range.len();
            let mut particle_object = ParticlesObject {
                num_particles: n,
                alembic_attributes: object.alembic_attributes.clone(),
                attributes: Default::default(),
            };

            struct Visitor<'a>(&'a mut ParticlesObject);
            impl attribute::Visitor for Visitor<'_> {
                type Output = Result<(), ParticleObjectError>;

                fn visit<A: attribute::Trait>(&mut self) -> Self::Output
                where
                    A::T: Value,
                {
                    self.0
                        .insert_attr::<A>(repeat_n(A::DEFAULT, self.0.num_particles))
                }
            }
            let mut visitor = Visitor(&mut particle_object);
            for key in attribute::Key::iter() {
                key.visit(&mut visitor)
                    .map_err(|error| error.attach_name(name))?;
            }

            Ok::<_, IoStateError>((name.clone(), particle_object))
        })
        .collect::<Result<_, _>>()?;

    let mut collider = Default::default();
    let squishy_volumes_file_frame::Collider {
        vertex_positions,
        triangle_frictions,
    } = &mut collider;

    vertex_positions.resize(input_reader.header().total_vertices, Default::default());
    triangle_frictions.resize(input_reader.header().total_triangles, Default::default());

    let mut io_state = IoState {
        time: 0.,
        animated_globals,
        particle_objects,
        collider,
        time_step_history: Vec::new(),
        bvh: None,
        grid_nodes: None,
    };

    drop(scope);

    let harness = harness.scope(
        "Applying initial bulk".to_string(),
        input_frame.bulk.len().max(1).try_into().unwrap(),
    )?;

    for bulk in input_frame.bulk {
        match bulk.meta.captured_attribute {
            BulkAttribute::Particles(attribute) => {
                let particles = io_state.get_particle_object_mut(&bulk.meta.object_name)?;
                apply_particle_bulk(particles, attribute, &bulk)
                    .map_err(|error| error.attach_name(&bulk.meta.object_name))?;
            }
            BulkAttribute::Collider(attribute) => {
                let ObjectCollider {
                    vertex_range,
                    triangle_range,
                    collider_id: _,
                } = input_reader
                    .header()
                    .get_collider_object(&bulk.meta.object_name)?;

                match attribute {
                    FrameBulkCollider::Position => {
                        io_state.collider.vertex_positions[vertex_range.clone()]
                            .copy_from_slice(bulk.assume_floats()?);
                    }
                    FrameBulkCollider::Triangles => {}
                    FrameBulkCollider::Friction => {
                        io_state.collider.triangle_frictions[triangle_range.clone()]
                            .copy_from_slice(bulk.assume_floats()?);
                    }
                }
            }
        }

        harness.step()?;
    }

    // TODO: verify?
    /*
    for (particle_index, parameters) in parameters.as_mut_slice()[particle_range.clone()]
        .iter_mut()
        .enumerate()
    {
        (|| {
            let flags = input_flags[particle_index];
            let flags =
                ParticleFlags::from_bits(flags).ok_or(InvalidParticle::UnknownFlags(flags))?;
            if !(flags.contains(ParticleFlags::IS_SOLID) ^ flags.contains(ParticleFlags::IS_FLUID))
            {
                return Err(InvalidParticle::SolidXorFluid);
            }

            parameters.initial_volume = (inv_scale * input_sizes[particle_index]).powi(3);
            parameters.mass = parameters.initial_volume * input_densities[particle_index];
            parameters.viscosity = flags
                .contains(ParticleFlags::USE_VISCOSITY)
                .then(|| {
                    Ok::<ViscosityParameters, InvalidParticle>(ViscosityParameters {
                        dynamic: input_viscosities_dynamic?[particle_index],
                        bulk: input_viscosities_bulk?[particle_index],
                    })
                })
                .transpose()?;

            parameters.specific = if flags.contains(ParticleFlags::IS_SOLID) {
                let youngs_modulus = input_youngs_moduluses?[particle_index];
                let poisson_ratio = input_poissons_ratios?[particle_index];
                SpecificParticleParameters::Solid {
                    mu: mu(youngs_modulus, poisson_ratio)?,
                    lambda: lambda(youngs_modulus, poisson_ratio)?,
                    sand_alpha: flags
                        .contains(ParticleFlags::USE_SAND_ALPHA)
                        .then(|| Ok::<f32, InvalidParticle>(input_sand_alphas?[particle_index]))
                        .transpose()?,
                }
            } else {
                let exponent = input_exponents?[particle_index] as i32;
                let bulk_modulus = input_bulk_moduluses?[particle_index];
                exponent_in_bounds(exponent)?;
                bulk_modulus_in_bounds(bulk_modulus)?;
                SpecificParticleParameters::Fluid {
                    exponent,
                    bulk_modulus,
                }
            };

            Ok(())
        })()
        .map_err(|source| InitializationError::InvalidParticle {
            name: name.clone(),
            particle_index,
            source,
        })?;
    }
    */

    Ok(io_state)
}

fn reject_empty_particle_objects(input_reader: &InputReader) -> Result<(), InitializationError> {
    let empty_object = input_reader
        .header()
        .objects
        .iter()
        .find(|(_, object)| {
            matches!(object, Object::Particles(particles) if particles.particle_range.is_empty())
        });
    match empty_object {
        Some((name, _)) => Err(InitializationError::EmptyParticleObject { name: name.clone() }),
        None => Ok(()),
    }
}

#[derive(Error, Debug)]
enum ApplyBulkError {
    #[error(transparent)]
    ParticleObject(#[from] ParticleObjectError),
    #[error(transparent)]
    FrameVerification(#[from] FrameVerificationError),
}

impl ApplyBulkError {
    fn attach_name(self, name: &str) -> InitializationError {
        match self {
            Self::ParticleObject(error) => error.attach_name(name).into(),
            Self::FrameVerification(error) => error.into(),
        }
    }
}

fn apply_particle_bulk(
    particles: &mut ParticlesObject,
    attribute: FrameBulkParticles,
    bulk: &OwnedFrameBulk,
) -> Result<(), ApplyBulkError> {
    match attribute {
        FrameBulkParticles::Flags => {
            (
                particles.view_attr_mut::<attribute::Flags>()?,
                bulk.assume_ints::<[u32; 2]>()?,
            )
                .into_par_iter()
                .for_each(|(fs, [mask, values])| {
                    let flags = ParticleFlags(two_f32_to_u32(*fs)).overwritten(*mask, *values);
                    *fs = u32_to_two_f32(flags.0);
                });
        }
        FrameBulkParticles::ColliderBits => {
            (
                particles.view_attr_mut::<attribute::ColliderBits>()?,
                bulk.assume_ints::<u32>()?,
            )
                .into_par_iter()
                .for_each(|(fs, bits)| *fs = u32_to_two_f32(*bits));
        }
        FrameBulkParticles::Transform => {
            let transforms: &[[[f32; 4]; 4]] = bulk.assume_floats()?;
            let mut positions = particles.remove_attr::<attribute::Position>()?;
            let mut position_gradients_x =
                particles.remove_attr::<attribute::PositionGradientX>()?;
            let mut position_gradients_y =
                particles.remove_attr::<attribute::PositionGradientY>()?;
            let mut position_gradients_z =
                particles.remove_attr::<attribute::PositionGradientZ>()?;

            (
                &mut positions,
                &mut position_gradients_x,
                &mut position_gradients_y,
                &mut position_gradients_z,
                transforms,
            )
                .into_par_iter()
                .for_each(
                    |(
                        position,
                        position_gradient_x,
                        position_gradient_y,
                        position_gradient_z,
                        [
                            [mxx, myx, mzx, _],
                            [mxy, myy, mzy, _],
                            [mxz, myz, mzz, _],
                            [px, py, pz, _],
                        ],
                    )| {
                        *position = [*px, *py, *pz];
                        *position_gradient_x = [*mxx, *myx, *mzx];
                        *position_gradient_y = [*mxy, *myy, *mzy];
                        *position_gradient_z = [*mxz, *myz, *mzz];
                    },
                );

            particles.insert_attr::<attribute::Position>(positions.into_iter())?;
            particles
                .insert_attr::<attribute::PositionGradientX>(position_gradients_x.into_iter())?;
            particles
                .insert_attr::<attribute::PositionGradientY>(position_gradients_y.into_iter())?;
            particles
                .insert_attr::<attribute::PositionGradientZ>(position_gradients_z.into_iter())?;
        }
        FrameBulkParticles::Size => {
            particles
                .replace_attr::<attribute::InitialSize>(bulk.assume_floats()?.iter().cloned())?;
        }
        FrameBulkParticles::Density => {
            particles.replace_attr::<attribute::Density>(bulk.assume_floats()?.iter().cloned())?;
        }
        FrameBulkParticles::YoungsModulus => {
            particles
                .replace_attr::<attribute::YoungsModulus>(bulk.assume_floats()?.iter().cloned())?;
        }
        FrameBulkParticles::PoissonsRatio => {
            particles
                .replace_attr::<attribute::PoissonsRatio>(bulk.assume_floats()?.iter().cloned())?;
        }
        FrameBulkParticles::InitialPosition => {
            particles.replace_attr::<attribute::InitialPosition>(
                bulk.assume_floats()?.iter().cloned(),
            )?;
        }
        FrameBulkParticles::Velocity => {
            particles.replace_attr::<attribute::Velocity>(bulk.assume_floats()?.iter().cloned())?;
        }
        FrameBulkParticles::ViscosityDynamic => {
            particles.replace_attr::<attribute::ViscosityDynamic>(
                bulk.assume_floats()?.iter().cloned(),
            )?;
        }
        FrameBulkParticles::ViscosityBulk => {
            particles
                .replace_attr::<attribute::ViscosityBulk>(bulk.assume_floats()?.iter().cloned())?;
        }
        FrameBulkParticles::Exponent => {
            particles.replace_attr::<attribute::Exponent>(
                bulk.assume_ints::<i32>()?.iter().map(|i| *i as f32),
            )?;
        }
        FrameBulkParticles::BulkModulus => {
            particles
                .replace_attr::<attribute::BulkModulus>(bulk.assume_floats()?.iter().cloned())?;
        }
        FrameBulkParticles::SandAlpha => {
            particles
                .replace_attr::<attribute::SandAlpha>(bulk.assume_floats()?.iter().cloned())?;
        }
        FrameBulkParticles::GoalPosition => {
            particles
                .replace_attr::<attribute::GoalPosition>(bulk.assume_floats()?.iter().cloned())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use squishy_volumes_api::InputBulk;
    use squishy_volumes_file_input::{
        FrameBulk, FrameBulkMeta, InputHeader, InputObject, InputObjectParticles, write_test_input,
    };
    use tempfile::TempDir;

    use super::*;

    fn particles(name: &str, num_particles: u32) -> (String, InputObject) {
        (
            name.to_string(),
            InputObject::Particles(InputObjectParticles {
                num_particles,
                alembic_attributes: Default::default(),
            }),
        )
    }

    fn initialization_error(
        objects: impl IntoIterator<Item = (String, InputObject)>,
    ) -> InitializationError {
        let dir = TempDir::new().unwrap();
        let mut input_reader = write_test_input(
            dir.path().join("input.bin"),
            InputHeader::test_input(objects),
            1,
        );
        let harness = Harness::new("test".to_string(), 1.try_into().unwrap());
        initialize_io_state(&harness, &mut input_reader)
            .err()
            .expect("initialization should fail")
    }

    #[test]
    fn empty_particle_object_next_to_filled_one_is_rejected() {
        let error = initialization_error([particles("filled", 3), particles("hollow", 0)]);
        assert!(matches!(
            error,
            InitializationError::EmptyParticleObject { name } if name == "hollow"
        ));
    }

    #[test]
    fn only_empty_particle_object_is_rejected() {
        let error = initialization_error([particles("empty", 0)]);
        assert!(matches!(
            error,
            InitializationError::EmptyParticleObject { name } if name == "empty"
        ));
    }

    #[test]
    fn particle_bulk_error_names_the_object() {
        let bulk: OwnedFrameBulk = FrameBulk {
            meta: FrameBulkMeta {
                object_name: "named_object".to_string(),
                captured_attribute: BulkAttribute::Particles(FrameBulkParticles::Size),
            },
            data: InputBulk::Floats(Cow::Owned(vec![1.; 3])),
        }
        .into();

        let error = apply_particle_bulk(
            &mut ParticlesObject::default(),
            FrameBulkParticles::Size,
            &bulk,
        )
        .unwrap_err()
        .attach_name("named_object");

        assert!(matches!(
            error,
            InitializationError::IoState(IoStateError::ParticleObject { name, .. })
                if name == "named_object"
        ));
    }
}
