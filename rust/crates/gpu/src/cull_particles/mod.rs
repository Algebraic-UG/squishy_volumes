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

#[cfg(test)]
mod test;

use squishy_volumes_util::ParticleFlags;
use std::num::NonZeroU32;

use super::*;

pub struct CullParticles {
    cull_particles: CompiledModule,

    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub domain_min: nalgebra::Vector3<f32>,
    pub domain_max: nalgebra::Vector3<f32>,
}

pub struct Parameters;

pub struct Input {
    pub particle_flags: Allocation,
    pub particle_positions_and_collider_bits: Allocation,
}

pub struct InputData<'a> {
    pub particle_flags: &'a [ParticleFlags],
    pub particle_positions_and_collider_bits: &'a [PositionAndColliderBits],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        InputData {
            particle_flags,
            particle_positions_and_collider_bits,
        }: InputData,
    ) -> Result<Self, GpuError> {
        check_length!(particle_flags, particle_positions_and_collider_bits)?;

        let particle_flags = Allocation::new(device, "particle_flags", particle_flags)?;
        let particle_positions_and_collider_bits = Allocation::new(
            device,
            "particle_positions_and_collider_bits",
            particle_positions_and_collider_bits,
        )?;

        Ok(Self {
            particle_flags,
            particle_positions_and_collider_bits,
        })
    }
}

pub struct Output;

impl PipelinePart for CullParticles {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            domain_min,
            domain_max,
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            cull_particles,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (ParticleFlags::MIN_BINDING_SIZE, false),
                    (PositionAndColliderBits::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("DOMAIN_MIN_X", domain_min.x as f64),
                    ("DOMAIN_MIN_Y", domain_min.y as f64),
                    ("DOMAIN_MIN_Z", domain_min.z as f64),
                    ("DOMAIN_MAX_X", domain_max.x as f64),
                    ("DOMAIN_MAX_Y", domain_max.y as f64),
                    ("DOMAIN_MAX_Z", domain_max.z as f64),
                ]
            }
        );

        Ok(Self {
            cull_particles,
            workgroup_size,
            dispatch_limit,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            particle_flags,
            particle_positions_and_collider_bits,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: particle_flags.len::<ParticleFlags>().get() as u32,
        })
        .direct();

        let mut compute_pass = context.enter_module(
            encoder,
            &self.cull_particles,
            [
                particle_flags.binding(),
                particle_positions_and_collider_bits.binding(),
            ],
        );
        compute_pass.dispatch_workgroups(x, y, z);

        Ok(Output)
    }
}
