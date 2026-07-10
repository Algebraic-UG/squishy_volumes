// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Matrix3;
use squishy_volumes_file_frame::{
    IoState, ParticleFlags, SpecificParticleParameters, ViscosityParameters,
};
use squishy_volumes_file_input::{InputRange, InputRanges, InputReader};
use squishy_volumes_xpu::Harness;
use thiserror::Error;

use squishy_volumes_util::{bulk_modulus_in_bounds, exponent_in_bounds, lambda, mu};

#[derive(Error, Debug)]
pub enum StateInitializationError {
    #[error("Harness error: {0}")]
    HarnessError(#[from] squishy_volumes_xpu::HarnessError),

    #[error("The object is missing in the header: {0}")]
    ObjectMissing(String),
    #[error("The object's type doesn't match the one in the header: {0}")]
    ObjectTypeMismatch(String),

    #[error("'{name}': input particle #{particle_index} invalid: {error}")]
    ParticleInvalid {
        name: String,
        particle_index: usize,
        error: ParticleInvalid,
    },
    #[error("Expected {expected} values for {name}, but found {actual}")]
    ParticleInvalidNumber {
        name: &'static str,
        actual: usize,
        expected: usize,
    },

    #[error("Failed to read input for state initialization: {0}")]
    InpuError(#[from] squishy_volumes_file_input::InputError),
}

#[derive(Error, Debug)]
pub enum ParticleInvalid {
    #[error("Some flags are set that are not know: {0:b}")]
    UnknownFlagsSet(u32),

    #[error("The particle solid or fluid flag must be set, but not both")]
    SolidXorFluid,

    #[error("Energy error: {0}")]
    EnergyError(#[from] squishy_volumes_util::EnergyError),
}

pub fn initialize_io_state(
    harness: &Harness,
    input_reader: &mut InputReader,
) -> Result<IoState, StateInitializationError> {
    let (input_header, input_frame) = {
        let _scope = harness.scope("Input reading".to_string(), 1.try_into().unwrap())?;
        (input_reader.read_header()?, input_reader.read_frame(0)?)
    };
    let input_ranges = InputRanges::new(&input_header.objects);

    harness.check()?;
    let mut io_state = IoState::default();
    {
        let _scope = harness.scope("Allocating Objects".to_string(), 1.try_into().unwrap())?;
        let n = input_header.total_particles();

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

        flags.resize(n, Default::default());
        parameters.resize(n, Default::default());
        elastic_energies.resize(n, Default::default());
        collider_bits.resize(n, Default::default());
        positions.resize(n, Default::default());
        position_gradients.resize(n, Matrix3::identity().into());
        velocities.resize(n, Default::default());
        velocity_gradients.resize(n, Default::default());
        initial_positions.resize(n, Default::default());
    }

    let harness = harness.scope(
        "Initializing Objects".to_string(),
        input_frame
            .particles_inputs
            .len()
            .max(1)
            .try_into()
            .unwrap(),
    )?;
    for (name, input) in input_frame.particles_inputs {
        harness.check()?;

        let InputRange::Particles { particle_range } = input_ranges
            .objects
            .get(&name)
            .ok_or(StateInitializationError::ObjectMissing(name.clone()))?
        else {
            return Err(StateInitializationError::ObjectTypeMismatch(name.clone()));
        };

        let squishy_volumes_file_frame::Particles {
            flags,
            parameters,
            elastic_energies: _, // TODO
            collider_bits: _,    // TODO
            positions,
            position_gradients,
            velocities,
            velocity_gradients: _, // TODO
            initial_positions,
        } = &mut io_state.particles;

        flags.as_mut_slice()[particle_range.clone()]
            .copy_from_slice(bytemuck::cast_slice(&input.flags));
        for (particle_index, parameters) in parameters.as_mut_slice()[particle_range.clone()]
            .iter_mut()
            .enumerate()
        {
            (|| {
                let flags = input.flags[particle_index];
                let flags = ParticleFlags::from_bits(flags)
                    .ok_or(ParticleInvalid::UnknownFlagsSet(flags))?;
                if !(flags.contains(ParticleFlags::IS_SOLID)
                    ^ flags.contains(ParticleFlags::IS_FLUID))
                {
                    return Err(ParticleInvalid::SolidXorFluid);
                }

                parameters.initial_volume =
                    (input_header.consts.simulation_scale * input.sizes[particle_index]).powi(3);
                parameters.mass = parameters.initial_volume * input.densities[particle_index];
                parameters.viscosity =
                    flags
                        .contains(ParticleFlags::USE_VISCOSITY)
                        .then(|| ViscosityParameters {
                            dynamic: input.viscosities_dynamic[particle_index],
                            bulk: input.viscosities_bulk[particle_index],
                        });

                parameters.specific = if flags.contains(ParticleFlags::IS_SOLID) {
                    let youngs_modulus = input.youngs_moduluses[particle_index];
                    let poisson_ratio = input.poissons_ratios[particle_index];
                    SpecificParticleParameters::Solid {
                        mu: mu(youngs_modulus, poisson_ratio)?,
                        lambda: lambda(youngs_modulus, poisson_ratio)?,
                        sand_alpha: flags
                            .contains(ParticleFlags::USE_SAND_ALPHA)
                            .then(|| input.sand_alphas[particle_index]),
                    }
                } else {
                    let exponent = input.exponents[particle_index] as i32;
                    let bulk_modulus = input.bulk_moduluses[particle_index];
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

        positions.as_mut_slice()[particle_range.clone()]
            .iter_mut()
            .zip(position_gradients.as_mut_slice()[particle_range.clone()].iter_mut())
            .zip(input.transforms)
            .for_each(|((position, position_gradient), transform)| {
                *position_gradient = [
                    [transform[0][0], transform[0][1], transform[0][2]], //
                    [transform[1][0], transform[1][1], transform[1][2]], //
                    [transform[2][0], transform[2][1], transform[2][2]], //
                ];
                *position = [transform[3][0], transform[3][1], transform[3][2]];
            });

        velocities.as_mut_slice()[particle_range.clone()]
            .copy_from_slice(&input.initial_velocities);

        initial_positions.as_mut_slice()[particle_range.clone()]
            .copy_from_slice(&input.initial_positions);

        harness.step()?;
    }

    Ok(io_state)
}
