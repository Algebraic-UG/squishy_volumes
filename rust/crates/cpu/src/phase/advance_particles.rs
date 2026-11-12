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

use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use squishy_volumes_util::{
    ParticleFlags, elastic_energy_inviscid, fluid_plasticity, profile, sand_plasticity,
    try_elastic_energy_neo_hookean,
};

use super::*;

impl CpuState {
    pub fn advance_particles(&mut self) -> Result<(), Error> {
        profile!("advance_particles");

        let time_step = self.adaptive_time_step_state.allowed_time_step();
        self.particles
            .elastic_energies
            .par_iter_mut()
            .zip(&self.particles.parameters)
            .zip(&mut self.particles.positions)
            .zip(&mut self.particles.position_gradients)
            .zip(&self.particles.velocities)
            .zip(&self.particles.velocity_gradients)
            .zip(&mut self.particles.flags)
            .filter(|(_, flags)| !flags.contains(ParticleFlags::TOMBSTONED))
            .try_for_each(
                |(
                    (
                        ((((elastic_energy, parameters), position), position_gradient), velocity),
                        velocity_gradient,
                    ),
                    flags,
                )|
                 -> Result<(), Error> {
                    *position += velocity * time_step;
                    *position_gradient += velocity_gradient * *position_gradient * time_step;

                    *elastic_energy = if flags.contains(ParticleFlags::IS_SOLID) {
                        if flags.contains(ParticleFlags::USE_SAND_ALPHA) {
                            *position_gradient = sand_plasticity(
                                parameters.mu(),
                                parameters.lambda(),
                                parameters.sand_alpha,
                                position_gradient,
                            );
                        }

                        try_elastic_energy_neo_hookean(
                            parameters.mu(),
                            parameters.lambda(),
                            position_gradient,
                        )
                        .inspect_err(|_| *flags |= ParticleFlags::FAILED)?
                    } else if flags.contains(ParticleFlags::IS_FLUID) {
                        *position_gradient = fluid_plasticity(position_gradient);
                        elastic_energy_inviscid(
                            parameters.bulk_modulus,
                            parameters.exponent,
                            position_gradient,
                        )
                    } else {
                        unreachable!()
                    };
                    Ok(())
                },
            )?;

        Ok(())
    }
}
