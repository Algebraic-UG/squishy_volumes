// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator as _, ParallelIterator};
use squishy_volumes_file_frame::{SpecificParticleParameters, ViscosityParameters};
use squishy_volumes_util::{
    cauchy_stress_general_viscosity, first_piola_stress_inviscid, first_piola_stress_neo_hookean,
    profile,
};

use super::*;

impl CpuState {
    // Mass and velocity transported by particles is scattered to the grids.
    // In explicit time integration the forces can be applied at the same time.
    pub fn scatter_momentum(&mut self, grid_node_size: f32) {
        profile!("scatter_momentum");
        let scaling =
            self.adaptive_time_step_state.allowed_time_step() * 4. / grid_node_size.powi(2);

        self.grid_nodes.masses = vec![0.; self.grid_nodes.map.len()];
        self.grid_nodes.velocities = vec![Vector3::zeros(); self.grid_nodes.map.len()];
        self.grid_nodes
            .keys
            .par_iter()
            .zip(&mut self.grid_nodes.contributors)
            .zip(&mut self.grid_nodes.masses)
            .zip(&mut self.grid_nodes.velocities)
            .for_each(
                |(((GridKey { node_id, .. }, contributors), mass), velocity)| {
                    for &particle_idx in contributors.get_mut().unwrap().iter() {
                        let particle_idx = particle_idx as usize;
                        let normalized = self.particles.positions[particle_idx] / grid_node_size;

                        let to_grid_node_normalized = node_id.map(|x| x as f32) - normalized;
                        let weight = to_grid_node_normalized.map(kernel_quadratic).product();

                        let to_grid_node = to_grid_node_normalized * grid_node_size;

                        let parameters = self.particles.parameters[particle_idx];
                        let mut imparted_momentum = (self.particles.velocities[particle_idx]
                            + self.particles.velocity_gradients[particle_idx] * to_grid_node)
                            * parameters.mass;

                        let position_gradient = &self.particles.position_gradients[particle_idx];
                        let stress = match parameters.specific {
                            SpecificParticleParameters::Solid {
                                mu,
                                lambda,
                                sand_alpha: _,
                            } => first_piola_stress_neo_hookean(mu, lambda, position_gradient),
                            SpecificParticleParameters::Fluid {
                                exponent,
                                bulk_modulus,
                            } => first_piola_stress_inviscid(
                                bulk_modulus,
                                exponent,
                                position_gradient,
                            ),
                        };

                        if let Some(ViscosityParameters { dynamic, bulk }) = parameters.viscosity {
                            let cauchy_stress = cauchy_stress_general_viscosity(
                                dynamic,
                                bulk,
                                &self.particles.velocity_gradients[particle_idx],
                            );

                            imparted_momentum -= cauchy_stress
                                * (to_grid_node
                                    * (scaling
                                        * position_gradient.determinant()
                                        * parameters.initial_volume));
                        }

                        imparted_momentum -= stress
                            * (position_gradient.transpose()
                                * (to_grid_node * (scaling * parameters.initial_volume)));

                        imparted_momentum *= weight;

                        *mass += weight * parameters.mass;
                        *velocity += imparted_momentum;
                    }
                },
            );
    }
}
