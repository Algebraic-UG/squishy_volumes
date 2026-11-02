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

use nalgebra::Vector3;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use squishy_volumes_util::{
    ParticleFlags, elastic_energy_inviscid, profile, try_elastic_energy_neo_hookean,
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
                            let mut svd = position_gradient.svd(true, true);
                            let e = svd.singular_values.map(f32::ln);
                            let e_tr = e.sum();
                            let e_hat = e - Vector3::repeat(e_tr / 3.);
                            let e_hat_norm = e_hat.norm();
                            if e_tr < 0. && e_hat_norm > 0. {
                                assert!(parameters.mu() > 0.);
                                if e_hat_norm != 0. {
                                    let delta_gamma = e_hat_norm
                                        + (3. * parameters.lambda() + 2. * parameters.mu())
                                            / 2.
                                            / parameters.mu()
                                            * e_tr
                                            * parameters.sand_alpha;
                                    if delta_gamma > 0. {
                                        let big_h = e - delta_gamma / e_hat_norm * e_hat;
                                        svd.singular_values = big_h.map(f32::exp);

                                        *position_gradient = svd.recompose().unwrap();
                                    }
                                }
                            } else {
                                *position_gradient = svd.u.unwrap() * svd.v_t.unwrap();
                            }
                        }

                        try_elastic_energy_neo_hookean(
                            parameters.mu(),
                            parameters.lambda(),
                            position_gradient,
                        )
                        .inspect_err(|_| *flags |= ParticleFlags::FAILED)?
                    } else if flags.contains(ParticleFlags::IS_FLUID) {
                        let mut svd = position_gradient.svd(true, true);
                        svd.singular_values
                            .fill(svd.singular_values.product().powf(1. / 3.));
                        *position_gradient = svd.recompose().unwrap();
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
