// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use squishy_volumes_file_frame::ParticleFlags;
use squishy_volumes_util::profile;
use squishy_volumes_xpu::FrameInput;

use super::*;

impl CpuState {
    pub fn cull_particles(&mut self, frame_input: &FrameInput) {
        let domain_min: nalgebra::Vector3<f32> = frame_input.consts().domain_min.into();
        let domain_max: nalgebra::Vector3<f32> = frame_input.consts().domain_max.into();

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
