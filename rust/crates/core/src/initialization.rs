// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{collections::BTreeMap, iter::repeat_n};

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use squishy_volumes_file_frame::{
    IoState, IoStateError, ParticleObjectError, ParticlesObject, Value, two_f32_to_u32,
    u32_to_two_f32,
};
use squishy_volumes_file_input::{
    BulkAttribute, FrameBulkCollider, FrameBulkParticles, FrameVerifcationError, InputReader,
    Object, ObjectCollider,
};
use squishy_volumes_util::{ParticleFlags, attribute};
use squishy_volumes_xpu::Harness;
use strum::IntoEnumIterator as _;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StateInitializationError {
    #[error("Io state error")]
    IoState(#[from] IoStateError),

    #[error("Harness error")]
    HarnessError(#[from] squishy_volumes_xpu::HarnessError),

    #[error("'{name}': input particle #{particle_index} invalid")]
    ParticleInvalid {
        name: String,
        particle_index: usize,
        #[source]
        error: ParticleInvalid,
    },
    #[error("Expected {expected} values for {name}, but found {actual}")]
    ParticleInvalidNumber {
        name: &'static str,
        actual: usize,
        expected: usize,
    },

    #[error("Failed to read frame for state initialization")]
    InputErrorFrame(#[from] squishy_volumes_file_input::InputError),
}

impl From<FrameVerifcationError> for StateInitializationError {
    fn from(error: FrameVerifcationError) -> Self {
        error.attach_frame(0).into()
    }
}

#[derive(Error, Debug, Clone, Copy)]
pub enum ParticleInvalid {
    #[error("This setup requires some input for {attribute}")]
    MissingInput { attribute: &'static str },

    #[error("Some flags are set that are not know: {0:b}")]
    UnknownFlagsSet(u32),

    #[error("The particle solid or fluid flag must be set, but not both")]
    SolidXorFluid,

    #[error("Energy error")]
    EnergyError(#[from] squishy_volumes_util::EnergyError),
}

pub fn initialize_io_state(
    harness: &Harness,
    input_reader: &mut InputReader,
) -> Result<IoState, StateInitializationError> {
    let input_frame = {
        let _scope = harness.scope("Input reading".to_string(), 1.try_into().unwrap())?;
        input_reader
            .read_frame(0)
            .map_err(StateInitializationError::InputErrorFrame)?
    };
    let animated_globals = input_frame.animated_globals;

    let scale = input_reader.header().consts.simulation_scale;
    let inv_scale = 1. / scale;

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
                    .map_err(|error| IoStateError::ParticleObject {
                        name: name.clone(),
                        error,
                    })?;
            }

            Ok::<_, IoStateError>((name.clone(), particle_object))
        })
        .collect::<Result<_, _>>()?;

    let mut collider = Default::default();
    let squishy_volumes_file_frame::Collider {
        vertex_positions,
        triangle_frictions,
        triangle_dampings,
    } = &mut collider;

    vertex_positions.resize(input_reader.header().total_vertices, Default::default());
    triangle_frictions.resize(input_reader.header().total_triangles, Default::default());
    triangle_dampings.resize(input_reader.header().total_triangles, Default::default());

    let mut io_state = IoState {
        time: 0.,
        animated_globals,
        particle_objects,
        collider,
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

                match attribute {
                    FrameBulkParticles::IsSolid
                    | FrameBulkParticles::IsFluid
                    | FrameBulkParticles::UseViscosity
                    | FrameBulkParticles::UseSandAlpha
                    | FrameBulkParticles::HasGoal
                    | FrameBulkParticles::IsActive => {
                        let flag = match attribute {
                            FrameBulkParticles::IsSolid => ParticleFlags::IS_SOLID,
                            FrameBulkParticles::IsFluid => ParticleFlags::IS_FLUID,
                            FrameBulkParticles::UseViscosity => ParticleFlags::USE_VISCOSITY,
                            FrameBulkParticles::UseSandAlpha => ParticleFlags::USE_SAND_ALPHA,
                            FrameBulkParticles::HasGoal => ParticleFlags::HAS_GOAL,
                            FrameBulkParticles::IsActive => todo!(),
                            _ => unreachable!(),
                        };
                        (
                            particles
                                .view_attr_mut::<attribute::Flags>()
                                .map_err(|e| e.attach_name(&bulk.meta.object_name))?,
                            bulk.assume_bools()?,
                        )
                            .into_par_iter()
                            .for_each(|(fs, bit)| {
                                let mut tmp = ParticleFlags(two_f32_to_u32(*fs));
                                tmp.set(flag, *bit);
                                *fs = u32_to_two_f32(tmp.0);
                            });
                    }
                    FrameBulkParticles::ColliderBits => {
                        (
                            particles
                                .view_attr_mut::<attribute::ColliderBits>()
                                .map_err(|e| e.attach_name(&bulk.meta.object_name))?,
                            bulk.assume_ints::<u32>()?,
                        )
                            .into_par_iter()
                            .for_each(|(fs, bits)| *fs = u32_to_two_f32(*bits));
                    }
                    FrameBulkParticles::Transforms => {
                        let transforms: &[[[f32; 4]; 4]] = bulk.assume_floats()?;
                        (|| {
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
                                        *position =
                                            [inv_scale * px, inv_scale * py, inv_scale * pz];
                                        *position_gradient_x = [*mxx, *myx, *mzx];
                                        *position_gradient_y = [*mxy, *myy, *mzy];
                                        *position_gradient_z = [*mxz, *myz, *mzz];
                                    },
                                );

                            particles.insert_attr::<attribute::Position>(positions.into_iter())?;
                            particles.insert_attr::<attribute::PositionGradientX>(
                                position_gradients_x.into_iter(),
                            )?;
                            particles.insert_attr::<attribute::PositionGradientY>(
                                position_gradients_y.into_iter(),
                            )?;
                            particles.insert_attr::<attribute::PositionGradientZ>(
                                position_gradients_z.into_iter(),
                            )?;

                            Ok::<_, ParticleObjectError>(())
                        })()
                        .map_err(|e| e.attach_name(&bulk.meta.object_name))?;
                    }
                    FrameBulkParticles::Sizes => {
                        particles
                            .replace_attr::<attribute::InitialSize>(
                                bulk.assume_floats::<f32>()?
                                    .into_iter()
                                    .map(|f| f * inv_scale),
                            )
                            .map_err(|e| e.attach_name(&bulk.meta.object_name))?;
                    }
                    FrameBulkParticles::Densities => {
                        particles
                            .replace_attr::<attribute::Density>(
                                bulk.assume_floats()?.into_iter().cloned(),
                            )
                            .map_err(|e| e.attach_name(&bulk.meta.object_name))?;
                    }
                    FrameBulkParticles::YoungsModuluses => {
                        particles
                            .replace_attr::<attribute::YoungsModulus>(
                                bulk.assume_floats()?.into_iter().cloned(),
                            )
                            .map_err(|e| e.attach_name(&bulk.meta.object_name))?;
                    }
                    FrameBulkParticles::PoissonsRatios => {
                        particles
                            .replace_attr::<attribute::PoissonsRatio>(
                                bulk.assume_floats()?.into_iter().cloned(),
                            )
                            .map_err(|e| e.attach_name(&bulk.meta.object_name))?;
                    }
                    FrameBulkParticles::InitialPositions => {
                        particles
                            .replace_attr::<attribute::InitialPosition>(
                                bulk.assume_floats()?.into_iter().cloned(),
                            )
                            .map_err(|e| e.attach_name(&bulk.meta.object_name))?;
                    }
                    FrameBulkParticles::InitialVelocity => {
                        particles
                            .replace_attr::<attribute::Velocity>(
                                bulk.assume_floats()?.into_iter().cloned(),
                            )
                            .map_err(|e| e.attach_name(&bulk.meta.object_name))?;
                    }
                    FrameBulkParticles::ViscosityDynamic => {
                        particles
                            .replace_attr::<attribute::ViscosityDynamic>(
                                bulk.assume_floats()?.into_iter().cloned(),
                            )
                            .map_err(|e| e.attach_name(&bulk.meta.object_name))?;
                    }
                    FrameBulkParticles::ViscosityBulk => {
                        particles
                            .replace_attr::<attribute::ViscosityBulk>(
                                bulk.assume_floats()?.into_iter().cloned(),
                            )
                            .map_err(|e| e.attach_name(&bulk.meta.object_name))?;
                    }
                    FrameBulkParticles::Exponent => {
                        particles
                            .replace_attr::<attribute::Exponent>(
                                bulk.assume_ints::<i32>()?.into_iter().map(|i| *i as f32),
                            )
                            .map_err(|e| e.attach_name(&bulk.meta.object_name))?;
                    }
                    FrameBulkParticles::BulkModulus => {
                        particles
                            .replace_attr::<attribute::BulkModulus>(
                                bulk.assume_floats()?.into_iter().cloned(),
                            )
                            .map_err(|e| e.attach_name(&bulk.meta.object_name))?;
                    }
                    FrameBulkParticles::SandAlpha => {
                        particles
                            .replace_attr::<attribute::SandAlpha>(
                                bulk.assume_floats()?.into_iter().cloned(),
                            )
                            .map_err(|e| e.attach_name(&bulk.meta.object_name))?;
                    }
                    FrameBulkParticles::GoalPositions => {
                        particles
                            .replace_attr::<attribute::GoalPosition>(
                                bulk.assume_floats::<[f32; 3]>()?.into_iter().map(|p| {
                                    [p[0] * inv_scale, p[1] * inv_scale, p[2] * inv_scale]
                                }),
                            )
                            .map_err(|e| e.attach_name(&bulk.meta.object_name))?;
                    }
                }
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
                    FrameBulkCollider::VertexPositions => {
                        io_state.collider.vertex_positions[vertex_range.clone()]
                            .copy_from_slice(bulk.assume_floats()?);
                        io_state.collider.vertex_positions[vertex_range.clone()]
                            .iter_mut()
                            .for_each(|p| {
                                p[0] *= inv_scale;
                                p[1] *= inv_scale;
                                p[2] *= inv_scale;
                            });
                    }
                    FrameBulkCollider::Triangles => {}
                    FrameBulkCollider::TriangleFrictions => {
                        io_state.collider.triangle_frictions[triangle_range.clone()]
                            .copy_from_slice(bulk.assume_floats()?);
                    }
                    FrameBulkCollider::TriangleDampings => {
                        io_state.collider.triangle_dampings[triangle_range.clone()]
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
                ParticleFlags::from_bits(flags).ok_or(ParticleInvalid::UnknownFlagsSet(flags))?;
            if !(flags.contains(ParticleFlags::IS_SOLID) ^ flags.contains(ParticleFlags::IS_FLUID))
            {
                return Err(ParticleInvalid::SolidXorFluid);
            }

            parameters.initial_volume = (inv_scale * input_sizes[particle_index]).powi(3);
            parameters.mass = parameters.initial_volume * input_densities[particle_index];
            parameters.viscosity = flags
                .contains(ParticleFlags::USE_VISCOSITY)
                .then(|| {
                    Ok::<ViscosityParameters, ParticleInvalid>(ViscosityParameters {
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
                        .then(|| Ok::<f32, ParticleInvalid>(input_sand_alphas?[particle_index]))
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
        .map_err(|error| StateInitializationError::ParticleInvalid {
            name: name.clone(),
            particle_index,
            error,
        })?;
    }
    */

    Ok(io_state)
}
