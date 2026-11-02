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

use std::num::NonZeroU32;

use nalgebra::Matrix4x3;
use squishy_volumes_util::{ParticleFlags, ParticleParameters};

use super::*;

pub struct Elastic {
    stress_and_energy: CompiledModule,
}

pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub position_gradients: Allocation,
    pub particle_flags: Allocation,
    pub particle_parameters: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        workgroup_size: NonZeroU32,
        dispatch_limit: NonZeroU32,
        position_gradients: &[Matrix4x3<f32>],
        particle_flags: &[ParticleFlags],
        particle_parameters: &[ParticleParameters],
    ) -> Result<Self, GpuError> {
        check_length!(position_gradients, particle_parameters)?;
        let indirect = Indirect::new(DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len: position_gradients.len() as u32,
        });

        let indirect = Allocation::new(device, "indirect", &[indirect])?;
        let position_gradients = Allocation::new(device, "position_gradients", position_gradients)?;
        let particle_flags = Allocation::new(device, "particle_flags", particle_flags)?;
        let particle_parameters =
            Allocation::new(device, "particle_parameters", particle_parameters)?;

        Ok(Self {
            indirect,
            position_gradients,
            particle_flags,
            particle_parameters,
        })
    }
}

pub struct Output {
    pub stresses: Allocation,
    pub energies: Allocation,
}

impl PipelinePart for Elastic {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings { workgroup_size }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            stress_and_energy,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),
                    (ParticleFlags::MIN_BINDING_SIZE, false),
                    (ParticleParameters::MIN_BINDING_SIZE, false),
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),
                    (f32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: []
            }
        );

        Ok(Self { stress_and_energy })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            position_gradients,
            particle_flags,
            particle_parameters,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let stresses = context
            .allocator()?
            .allocate::<Matrix4x3<f32>>("stresses", position_gradients.len::<Matrix4x3<f32>>())?;
        let energies = context
            .allocator()?
            .allocate::<f32>("energies", position_gradients.len::<Matrix4x3<f32>>())?;

        let mut compute_pass = context.enter_module(
            encoder,
            &self.stress_and_energy,
            [
                indirect.binding(),
                position_gradients.binding(),
                particle_flags.binding(),
                particle_parameters.binding(),
                stresses.binding(),
                energies.binding(),
            ],
        );
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());
        drop(compute_pass);

        Ok(Output { stresses, energies })
    }
}
