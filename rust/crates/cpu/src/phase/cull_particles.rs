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
use squishy_volumes_util::{ParticleFlags, profile};

use super::*;

impl CpuState {
    pub fn cull_particles(&mut self) {
        let domain_min: nalgebra::Vector3<f32> =
            self.frame_input.header().consts.scaled_domain_min().into();
        let domain_max: nalgebra::Vector3<f32> =
            self.frame_input.header().consts.scaled_domain_max().into();

        profile!("cull_particles");
        self.particles
            .flags
            .par_iter_mut()
            .zip(&self.particles.positions)
            .for_each(|(flags, position)| {
                if flags.contains(ParticleFlags::TOMBSTONED) {
                    return;
                }

                let within_bounds = position
                    .zip_zip_map(&domain_min, &domain_max, |p, min, max| p > min && p < max)
                    .iter()
                    .all(|b| *b);
                if within_bounds {
                    return;
                }

                *flags |= ParticleFlags::TOMBSTONED;
            });
    }
}
