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
use squishy_volumes_util::ParticleParameters;

use super::*;

pub struct Sand {
    sand: CompiledModule,
    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub particle_parameters: Allocation,
    pub particle_position_gradients: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        particle_parameters: &[ParticleParameters],
        particle_position_gradients: &[Matrix4x3<f32>],
    ) -> Result<Self, GpuError> {
        check_length!(particle_parameters, particle_position_gradients)?;
        let particle_parameters =
            Allocation::new(device, "particle_parameters", particle_parameters)?;
        let particle_position_gradients = Allocation::new(
            device,
            "particle_position_gradients",
            particle_position_gradients,
        )?;

        Ok(Self {
            particle_parameters,
            particle_position_gradients,
        })
    }
}

pub struct Output;

impl PipelinePart for Sand {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &mut GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
    ) -> Result<Self, GpuPipelineCreationError> {
        let_compiled_module!(
            sand,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (ParticleParameters::MIN_BINDING_SIZE, false),
                    (Matrix4x3::<f32>::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: []
            }
        );

        Ok(Self {
            sand,
            workgroup_size,
            dispatch_limit,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            particle_parameters,
            particle_position_gradients,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        assert_eq!(
            particle_parameters.len::<ParticleParameters>(),
            particle_position_gradients.len::<Matrix4x3<f32>>()
        );
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: particle_parameters.len::<ParticleParameters>().get() as u32,
        })
        .direct();

        context
            .enter_module(
                encoder,
                &self.sand,
                [
                    particle_parameters.binding(),
                    particle_position_gradients.binding(),
                ],
            )
            .dispatch_workgroups(x, y, z);

        Ok(Output)
    }
}
