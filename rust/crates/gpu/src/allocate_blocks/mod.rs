// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU32;

#[cfg(test)]
mod test;

use super::*;

pub struct AllocateBlocks {
    bits_to_pops: BitsToPops,
    prefix_sum: PrefixSum,
    len_to_indirect: LenToIndirect,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub owns: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
        owns: &[u32],
    ) -> Self {
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: owns.len() as u32,
        });

        let owns = Allocation::new(device, "owns", owns);
        let indirect = Allocation::new(device, "indirect", &[indirect]);

        Self { indirect, owns }
    }
}

pub struct Output {
    pub block_offsets: Allocation,
    pub indirect_blocks: Allocation,
}

impl PipelinePart for AllocateBlocks {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
    ) -> Self {
        let bits_to_pops = BitsToPops::new(context, bits_to_pops::Settings { workgroup_size });

        let prefix_sum = PrefixSum::new(
            context,
            prefix_sum::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );

        let len_to_indirect = LenToIndirect::new(
            context,
            len_to_indirect::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );

        Self {
            bits_to_pops,
            prefix_sum,
            len_to_indirect,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input { indirect, owns }: Input,
        _: Self::Parameters,
    ) -> Result<Output, GpuError> {
        let bits_to_pops::Output { pops } = self.bits_to_pops.record(
            context,
            encoder,
            bits_to_pops::Input {
                indirect: indirect.clone(),
                bits: owns,
            },
            bits_to_pops::Parameters,
        )?;

        let prefix_sum::Output {
            prefix_sums: block_offsets,
            total_sum,
        } = self.prefix_sum.record(
            context,
            encoder,
            prefix_sum::Input {
                indirect,
                numbers: pops,
            },
            prefix_sum::Parameters { total_sum: true },
        )?;

        let len_to_indirect::Output {
            new_indirect: indirect_blocks,
            ..
        } = self.len_to_indirect.record(
            context,
            encoder,
            len_to_indirect::Input {
                len: total_sum.unwrap(),
            },
            len_to_indirect::Parameters,
        )?;

        Ok(Output {
            block_offsets,
            indirect_blocks,
        })
    }
}
