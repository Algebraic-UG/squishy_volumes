// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use squishy_volumes_file_frame::ParticleFlags;
use squishy_volumes_util::profile;
use squishy_volumes_xpu::FrameInput;

use super::*;

impl CpuState {
    pub fn external_force(&mut self, frame_input: &FrameInput) -> Result<(), Error> {
        profile!("external_force");
        let time_step = self.adaptive_time_step_state.allowed_time_step();
        let input_flags = frame_input.a().particle_flags();
        let interpolated_input = self
            .interpolated_input
            .as_ref()
            .ok_or(Error::InterpolatedInputMissing)?;

        self.particles
            .positions
            .par_iter()
            .zip(&mut self.particles.velocities)
            .enumerate()
            .zip(&self.particles.flags)
            .filter_map(|(e, flags)| (!flags.contains(ParticleFlags::TOMBSTONED)).then_some(e))
            .for_each(|(index, (position, velocity))| {
                let index = self.particles.sort_map[index] as usize;
                if input_flags[index].contains(ParticleFlags::HAS_GOAL) {
                    *velocity =
                        (interpolated_input.particle_goal_positions[index] - position) / time_step;
                } else {
                    *velocity += time_step * interpolated_input.gravity;
                }
            });

        Ok(())
    }
}
