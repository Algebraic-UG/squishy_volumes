// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Matrix3;
use squishy_volumes_file_frame::IoState;
use squishy_volumes_file_input::{
    BulkAttribute, FrameBulkCollider, FrameBulkParticles, InputError, InputRangeCollider,
    InputRangeParticles, InputRanges, InputReader,
};
use squishy_volumes_xpu::Harness;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StateInitializationError {
    #[error("Harness error")]
    HarnessError(#[from] squishy_volumes_xpu::HarnessError),

    #[error("The object is missing in the header: {0}")]
    ObjectMissing(String),
    #[error("The object's type doesn't match the one in the header: {0}")]
    ObjectTypeMismatch(String),

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

    #[error("Failed to read input for state initialization")]
    InputError(#[from] squishy_volumes_file_input::InputError),

    #[error("'{name}': missing input for {attribute}")]
    MissingInput {
        name: String,
        attribute: &'static str,
    },

    // TODO: duplicate from frame_input
    #[error("Failed to cast data")]
    CastFailed(#[from] bytemuck::PodCastError),
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

macro_rules! object_missing_input {
    ($name:expr, $attribute:expr) => {
        $attribute.ok_or(StateInitializationError::MissingInput {
            name: $name.clone(),
            attribute: stringify!($attribute),
        })
    };
}
macro_rules! particle_missing_input {
    ($name:expr) => {
        $name.as_ref().ok_or(ParticleInvalid::MissingInput {
            attribute: stringify!($name),
        })
    };
}

pub fn initialize_io_state(
    harness: &Harness,
    input_reader: &mut InputReader,
) -> Result<IoState, StateInitializationError> {
    let input_frame = {
        let _scope = harness.scope("Input reading".to_string(), 1.try_into().unwrap())?;
        input_reader.read_frame(0)?
    };
    let input_ranges = InputRanges::new(&input_reader.header().objects);

    let scale = input_reader.header().consts.simulation_scale;
    let inv_scale = 1. / scale;

    harness.check()?;
    let mut io_state = IoState::default();
    let scope = harness.scope("Allocating Objects".to_string(), 1.try_into().unwrap())?;

    let squishy_volumes_file_frame::Particles {
        flags,
        parameters,
        elastic_energies,
        collider_bits,
        positions,
        position_gradients,
        velocities,
        velocity_gradients,
        initial_positions,
    } = &mut io_state.particles;

    let total_particles = input_ranges.total_particles;
    flags.resize(total_particles, Default::default());
    parameters.resize(total_particles, Default::default());
    elastic_energies.resize(total_particles, Default::default());
    collider_bits.resize(total_particles, Default::default());
    positions.resize(total_particles, Default::default());
    position_gradients.resize(total_particles, Matrix3::identity().into());
    velocities.resize(total_particles, Default::default());
    velocity_gradients.resize(total_particles, Default::default());
    initial_positions.resize(total_particles, Default::default());

    io_state
        .goal_positions
        .resize(total_particles, Default::default());

    let squishy_volumes_file_frame::Collider {
        vertex_positions,
        triangle_frictions,
        triangle_dampings,
    } = &mut io_state.collider;

    vertex_positions.resize(input_ranges.total_vertices, Default::default());
    triangle_frictions.resize(input_ranges.total_triangles, Default::default());
    triangle_dampings.resize(input_ranges.total_triangles, Default::default());

    drop(scope);

    let harness = harness.scope(
        "Applying initial bulk".to_string(),
        input_frame.bulk.len().max(1).try_into().unwrap(),
    )?;
    for bulk in input_frame.bulk {
        match bulk.meta.captured_attribute {
            BulkAttribute::Particles(attribute) => {
                let InputRangeParticles { particle_range } = input_ranges
                    .get_particle_range(&bulk.meta.object_name)
                    .map_err(|error| InputError::FrameVerifcationError {
                        frame: 0,
                        error: error.into(),
                    })?;
                match attribute {
                    FrameBulkParticles::Flags => {
                        flags[particle_range].copy_from_slice(bulk.data.assume_ints()?)
                    }
                    FrameBulkParticles::ColliderBits => {
                        collider_bits[particle_range].copy_from_slice(bulk.data.assume_ints()?)
                    }
                    FrameBulkParticles::Transforms => {
                        let transforms: &[[[f32; 4]; 4]] = bulk.data.assume_floats()?;
                        for (i, m) in particle_range.into_iter().zip(transforms) {
                            positions[i] = [
                                inv_scale * m[3][0],
                                inv_scale * m[3][1],
                                inv_scale * m[3][2],
                            ];
                            position_gradients[i] = [
                                [m[0][0], m[0][1], m[0][2]], //
                                [m[1][0], m[1][1], m[1][2]], //
                                [m[2][0], m[2][1], m[2][2]], //
                            ];
                        }
                    }
                    FrameBulkParticles::Sizes => {
                        for (i, v) in particle_range
                            .into_iter()
                            .zip(bulk.data.assume_floats::<f32>()?)
                        {
                            parameters[i].initial_volume = (inv_scale * v).powi(3);
                        }
                    }
                    FrameBulkParticles::Densities => {
                        for (i, v) in particle_range
                            .into_iter()
                            .zip(bulk.data.assume_floats::<f32>()?)
                        {
                            parameters[i].density = *v;
                        }
                    }
                    FrameBulkParticles::YoungsModuluses => {
                        for (i, v) in particle_range
                            .into_iter()
                            .zip(bulk.data.assume_floats::<f32>()?)
                        {
                            parameters[i].youngs_modulus = *v;
                        }
                    }
                    FrameBulkParticles::PoissonsRatios => {
                        for (i, v) in particle_range
                            .into_iter()
                            .zip(bulk.data.assume_floats::<f32>()?)
                        {
                            parameters[i].poissons_ratio = *v;
                        }
                    }
                    FrameBulkParticles::InitialPositions => initial_positions[particle_range]
                        .copy_from_slice(bulk.data.assume_floats()?),
                    FrameBulkParticles::InitialVelocity => {
                        velocities[particle_range].copy_from_slice(bulk.data.assume_floats()?)
                    }
                    FrameBulkParticles::ViscosityDynamic => {
                        for (i, v) in particle_range
                            .into_iter()
                            .zip(bulk.data.assume_floats::<f32>()?)
                        {
                            parameters[i].viscosity_dynamic = *v;
                        }
                    }
                    FrameBulkParticles::ViscosityBulk => {
                        for (i, v) in particle_range
                            .into_iter()
                            .zip(bulk.data.assume_floats::<f32>()?)
                        {
                            parameters[i].viscosity_bulk = *v;
                        }
                    }
                    FrameBulkParticles::Exponent => {
                        for (i, v) in particle_range
                            .into_iter()
                            .zip(bulk.data.assume_floats::<i32>()?)
                        {
                            parameters[i].exponent = *v;
                        }
                    }
                    FrameBulkParticles::BulkModulus => {
                        for (i, v) in particle_range
                            .into_iter()
                            .zip(bulk.data.assume_floats::<f32>()?)
                        {
                            parameters[i].bulk_modulus = *v;
                        }
                    }
                    FrameBulkParticles::SandAlpha => {
                        for (i, v) in particle_range
                            .into_iter()
                            .zip(bulk.data.assume_floats::<f32>()?)
                        {
                            parameters[i].sand_alpha = *v;
                        }
                    }
                    FrameBulkParticles::GoalPositions => {}
                }
            }
            BulkAttribute::Collider(attribute) => {
                let InputRangeCollider {
                    vertex_range,
                    triangle_range,
                } = input_ranges
                    .get_collider_range(&bulk.meta.object_name)
                    .map_err(|error| InputError::FrameVerifcationError {
                        frame: 0,
                        error: error.into(),
                    })?;
                match attribute {
                    FrameBulkCollider::VertexPositions => {
                        vertex_positions[vertex_range].copy_from_slice(bulk.data.assume_floats()?);
                    }
                    FrameBulkCollider::Triangles => {}
                    FrameBulkCollider::TriangleFrictions => {
                        triangle_frictions[triangle_range]
                            .copy_from_slice(bulk.data.assume_floats()?);
                    }
                    FrameBulkCollider::TriangleDampings => {
                        triangle_dampings[triangle_range]
                            .copy_from_slice(bulk.data.assume_floats()?);
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

    io_state.grid_nodes = Some(Default::default());

    Ok(io_state)
}
