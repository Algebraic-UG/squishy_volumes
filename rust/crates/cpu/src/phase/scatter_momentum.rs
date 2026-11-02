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
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator as _, ParallelIterator};
use squishy_volumes_util::{
    ParticleFlags, cauchy_stress_general_viscosity, first_piola_stress_inviscid,
    first_piola_stress_neo_hookean, profile,
};

use super::*;

impl CpuState {
    // Mass and velocity transported by particles is scattered to the grids.
    // In explicit time integration the forces can be applied at the same time.
    pub fn scatter_momentum(&mut self) {
        profile!("scatter_momentum");
        let grid_node_size = self.frame_input.header().consts.scaled_grid_node_size();
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

                        let flags = self.particles.flags[particle_idx];
                        let parameters = self.particles.parameters[particle_idx];
                        let mut imparted_momentum = (self.particles.velocities[particle_idx]
                            + self.particles.velocity_gradients[particle_idx] * to_grid_node)
                            * parameters.mass();

                        let position_gradient = &self.particles.position_gradients[particle_idx];
                        let stress = if flags.contains(ParticleFlags::IS_SOLID) {
                            first_piola_stress_neo_hookean(
                                parameters.mu(),
                                parameters.lambda(),
                                position_gradient,
                            )
                        } else if flags.contains(ParticleFlags::IS_FLUID) {
                            first_piola_stress_inviscid(
                                parameters.bulk_modulus,
                                parameters.exponent,
                                position_gradient,
                            )
                        } else {
                            unreachable!()
                        };

                        if flags.contains(ParticleFlags::USE_VISCOSITY) {
                            let cauchy_stress = cauchy_stress_general_viscosity(
                                parameters.viscosity_dynamic,
                                parameters.viscosity_bulk,
                                &self.particles.velocity_gradients[particle_idx],
                            );

                            imparted_momentum -= cauchy_stress
                                * (to_grid_node
                                    * (scaling
                                        * position_gradient.determinant()
                                        * parameters.initial_volume()));
                        }

                        imparted_momentum -= stress
                            * (position_gradient.transpose()
                                * (to_grid_node * (scaling * parameters.initial_volume())));

                        imparted_momentum *= weight;

                        *mass += weight * parameters.mass();
                        *velocity += imparted_momentum;
                    }
                },
            );
    }
}
