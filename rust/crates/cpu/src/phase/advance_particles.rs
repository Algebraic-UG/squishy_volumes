// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use squishy_volumes_file_frame::{ParticleFlags, SpecificParticleParameters};
use squishy_volumes_util::{elastic_energy_inviscid, profile, try_elastic_energy_neo_hookean};

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
            .zip(&self.particles.flags)
            .filter_map(|(e, flags)| (!flags.contains(ParticleFlags::TOMBSTONED)).then_some(e))
            .try_for_each(
                |(
                    ((((elastic_energy, parameters), position), position_gradient), velocity),
                    velocity_gradient,
                )|
                 -> Result<(), Error> {
                    *position += velocity * time_step;
                    *position_gradient += velocity_gradient * *position_gradient * time_step;

                    *elastic_energy = match parameters.specific {
                        SpecificParticleParameters::Solid {
                            mu,
                            lambda,
                            sand_alpha,
                            ..
                        } => {
                            if let Some(alpha) = sand_alpha {
                                let mut svd = position_gradient.svd(true, true);
                                let e = svd.singular_values.map(f32::ln);
                                let e_tr = e.sum();
                                let e_hat = e - Vector3::repeat(e_tr / 3.);
                                let e_hat_norm = e_hat.norm();
                                if e_tr < 0. && e_hat_norm > 0. {
                                    assert!(mu > 0.);
                                    if e_hat_norm != 0. {
                                        let delta_gamma = e_hat_norm
                                            + (3. * lambda + 2. * mu) / 2. / mu * e_tr * alpha;
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

                            try_elastic_energy_neo_hookean(mu, lambda, position_gradient)?
                        }
                        SpecificParticleParameters::Fluid {
                            exponent,
                            bulk_modulus,
                            ..
                        } => {
                            let mut svd = position_gradient.svd(true, true);
                            svd.singular_values
                                .fill(svd.singular_values.product().powf(1. / 3.));
                            *position_gradient = svd.recompose().unwrap();
                            elastic_energy_inviscid(bulk_modulus, exponent, position_gradient)
                        }
                    };
                    Ok(())
                },
            )?;

        Ok(())
    }
}
