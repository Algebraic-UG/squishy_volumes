// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Context, Result};
use itertools::izip;
use nalgebra::Matrix3;
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::simulation::{
    elastic::{elastic_energy_inviscid, try_elastic_energy_neo_hookean},
    particles::ParticleParameters,
};

use super::{PhaseInput, State, profile};

impl State {
    pub(super) fn advect_particles(mut self, phase_input: PhaseInput) -> Result<Self> {
        profile!("advect_particles");
        let time_step = phase_input.time_step;

        izip!(
            self.particles.elastic_energies.iter_mut(),
            self.particles.parameters.iter(),
            self.particles.positions.iter_mut(),
            self.particles.position_gradients.iter_mut(),
            self.particles.velocities.iter(),
            self.particles.velocity_gradients.iter(),
        )
        .par_bridge()
        .try_for_each(
            |(
                elastic_energy,
                parameters,
                position,
                position_gradient,
                velocity,
                velocity_gradient,
            )|
             -> Result<()> {
                *position += velocity * time_step;
                *position_gradient += velocity_gradient * *position_gradient * time_step;
                *elastic_energy = match parameters {
                    ParticleParameters::Solid { mu, lambda } => {
                        try_elastic_energy_neo_hookean(*mu, *lambda, position_gradient)
                            .context("calculating new elastic energy")?
                    }
                    ParticleParameters::Fluid {
                        exponent,
                        bulk_modulus,
                    } => {
                        *position_gradient = Matrix3::from_diagonal_element(
                            position_gradient.determinant().powf(1. / 3.),
                        );
                        elastic_energy_inviscid(*bulk_modulus, *exponent, position_gradient)
                    }
                };
                Ok(())
            },
        )?;

        Ok(self)
    }
}
