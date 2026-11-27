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

use super::*;

pub struct WriteWords {
    write_words: CompiledModule,
    write_words_masked: CompiledModule,
    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Parameters {
    pub offset: u32,
    pub stride: u32,
    pub first: u32,
    pub words: u32,
}

pub struct Input {
    pub source: Allocation,
    pub mask: Option<Allocation>,
    pub target: Allocation,
}

pub struct Output;

impl PipelinePart for WriteWords {
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
    ) -> Result<Self, PipelineError> {
        let_compiled_module!(
            write_words,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: size_of::<Parameters>() as u32,
                constants: []
            }
        );
        let_compiled_module!(
            write_words_masked,
            CompiledModuleSettings {
                context,
                workgroup_size,
                bind_group_entries: [
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: size_of::<Parameters>() as u32,
                constants: []
            }
        );

        Ok(Self {
            write_words,
            write_words_masked,
            workgroup_size,
            dispatch_limit,
        })
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            source,
            mask,
            target,
        }: Input,
        parameters: Parameters,
    ) -> Result<Output, Error> {
        assert!(parameters.words > 0 && parameters.first + parameters.words <= parameters.stride);
        let num_source = source.len::<u32>().get() / parameters.words as u64;
        let num_target = target.len::<u32>().get() / parameters.stride as u64;
        if num_source + parameters.offset as u64 > num_target {
            Err(InputError::RangeDoesNotFit {
                a: "source",
                a_len: num_source as usize,
                b: "target",
                b_len: num_target as usize,
                offset: parameters.offset as usize,
            })?;
        }
        if let Some(mask) = &mask
            && mask.len::<u32>() != source.len::<u32>()
        {
            Err(InputError::LengthMismatch {
                a: "mask",
                a_len: mask.len::<u32>().get() as usize,
                b: "source",
                b_len: source.len::<u32>().get() as usize,
            })?;
        }
        let [x, y, z] = Indirect::new(DispatchSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: self.dispatch_limit,
            len: num_source as u32,
        })
        .direct();

        let mut compute_pass = match &mask {
            None => context.enter_module(
                encoder,
                &self.write_words,
                [source.binding(), target.binding()],
            ),
            Some(mask) => context.enter_module(
                encoder,
                &self.write_words_masked,
                [source.binding(), mask.binding(), target.binding()],
            ),
        };
        compute_pass.set_immediates(0, bytemuck::bytes_of(&parameters));
        compute_pass.dispatch_workgroups(x, y, z);

        Ok(Output)
    }
}
