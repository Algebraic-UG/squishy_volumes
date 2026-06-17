// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use nalgebra::Vector3;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator as _, ParallelIterator};
use squishy_volumes_api::T;
use squishy_volumes_util::{
    cauchy_stress_general_viscosity, first_piola_stress_inviscid, first_piola_stress_neo_hookean,
};

use crate::{
    kernels::kernel_quadratic,
    profile,
    state::{
        grids::GridKey,
        particles::{ParticleParameters, ViscosityParameters},
    },
};

use super::{PhaseInput, State};

impl State {
    // Mass and velocity transported by particles is scattered to the grids.
    // In explicit time integration the forces can be applied at the same time.
    pub fn scatter_momentum<const EXPLICIT_FORCES: bool>(
        mut self,
        phase_input: &mut PhaseInput,
    ) -> Result<Self> {
        profile!("scatter_momentum");
        let grid_node_size = phase_input.consts.scaled_grid_node_size();
        let scaling = phase_input.time_step * 4. / grid_node_size.powi(2);

        self.grid.masses = vec![0.; self.grid.map.len()];
        self.grid.velocities = vec![Vector3::zeros(); self.grid.map.len()];
        self.grid
            .keys
            .par_iter()
            .zip(&mut self.grid.contributors)
            .zip(&mut self.grid.masses)
            .zip(&mut self.grid.velocities)
            .for_each(
                |(((GridKey { node_id, .. }, contributors), mass), velocity)| {
                    for &particle_idx in contributors.get_mut().unwrap().iter() {
                        let particle_idx = particle_idx as usize;
                        let normalized = self.particles.positions[particle_idx] / grid_node_size;

                        let to_grid_node_normalized = node_id.map(|x| x as T) - normalized;
                        let weight = to_grid_node_normalized.map(kernel_quadratic).product();

                        let to_grid_node = to_grid_node_normalized * grid_node_size;

                        let mut imparted_momentum = (self.particles.velocities[particle_idx]
                            + self.particles.velocity_gradients[particle_idx] * to_grid_node)
                            * self.particles.masses[particle_idx];

                        if EXPLICIT_FORCES {
                            let position_gradient =
                                &self.particles.position_gradients[particle_idx];
                            let common_viscosity;
                            let stress = match self.particles.parameters[particle_idx] {
                                ParticleParameters::Solid {
                                    mu,
                                    lambda,
                                    viscosity,
                                    sand_alpha: _,
                                } => {
                                    common_viscosity = viscosity;
                                    first_piola_stress_neo_hookean(mu, lambda, position_gradient)
                                }
                                ParticleParameters::Fluid {
                                    exponent,
                                    bulk_modulus,
                                    viscosity,
                                } => {
                                    common_viscosity = viscosity;
                                    first_piola_stress_inviscid(
                                        bulk_modulus,
                                        exponent,
                                        position_gradient,
                                    )
                                }
                            };

                            if let Some(ViscosityParameters { dynamic, bulk }) = common_viscosity {
                                let cauchy_stress = cauchy_stress_general_viscosity(
                                    dynamic,
                                    bulk,
                                    &self.particles.velocity_gradients[particle_idx],
                                );

                                imparted_momentum -= cauchy_stress
                                    * (to_grid_node
                                        * (scaling
                                            * position_gradient.determinant()
                                            * self.particles.initial_volumes[particle_idx]));
                            }

                            imparted_momentum -= stress
                                * (position_gradient.transpose()
                                    * (to_grid_node
                                        * (scaling
                                            * self.particles.initial_volumes[particle_idx])));
                        }

                        imparted_momentum *= weight;

                        *mass += weight * self.particles.masses[particle_idx];
                        *velocity += imparted_momentum;
                    }
                },
            );

        Ok(self)
    }
}
