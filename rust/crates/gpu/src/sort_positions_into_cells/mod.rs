// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU32;

use nalgebra::Vector4;

use super::*;

#[cfg(test)]
mod test;

pub struct SortPositionsIntoCells {
    positions_to_keys: PositionsToKeys,
    radix_sort: RadixSort,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub cell_size: f32,
    pub bit_count: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub indices_in: Allocation,
    pub positions: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
            ..
        }: Settings,
        indices: &[u32],
        positions: &[Vector4<f32>],
    ) -> Self {
        assert_eq!(indices.len(), positions.len());
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: indices.len() as u32,
        });

        let indices_in = Allocation::new(device, "indices_in", indices);
        let positions = Allocation::new(device, "positions", positions);
        let indirect = Allocation::new(device, "indirect", &[indirect]);

        Self {
            indirect,
            indices_in,
            positions,
        }
    }
}

pub struct Output {
    indices_out: Allocation,
}

impl PipelinePart for SortPositionsIntoCells {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            cell_size,
            bit_count,
        }: Settings,
    ) -> Self {
        let positions_to_keys = PositionsToKeys::new(
            context,
            positions_to_keys::Settings {
                workgroup_size,
                cell_size,
            },
        );
        let radix_sort = RadixSort::new(
            context,
            radix_sort::Settings {
                workgroup_size,
                dispatch_limit,
                bit_count,
            },
        );

        Self {
            positions_to_keys,
            radix_sort,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            indices_in,
            positions,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let mut indices_out = indices_in;
        for dimension in [2, 1, 0] {
            let positions_to_keys::Output { keys } = self.positions_to_keys.record(
                context,
                encoder,
                positions_to_keys::Input {
                    indirect: indirect.clone(),
                    positions: positions.clone(),
                },
                positions_to_keys::Parameters { dimension },
            )?;
            indices_out = self.radix_sort.record_all_rounds(
                context,
                encoder,
                radix_sort::Input {
                    indirect: indirect.clone(),
                    indices_in: indices_out,
                    keys,
                },
            )?;
        }
        Ok(Output { indices_out })
    }
}
