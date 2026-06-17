// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use std::num::NonZeroU32;

use nalgebra::Vector4;

use super::*;

pub struct ColorCells {
    cells_to_colorkeys: CellsToColorkeys,
    radix_sort: RadixSort,
    recycle_to_indirect: RecycleToIndirect,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub cell_ids: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
        cell_ids: &[Vector4<i32>],
    ) -> Self {
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: cell_ids.len() as u32,
        });
        let indirect = Allocation::new(device, "indirect", &[indirect]);
        let cell_ids = Allocation::new(device, "cell_ids", cell_ids);
        Self { indirect, cell_ids }
    }
}

pub struct Output {
    pub indirect_colors: Allocation,
    pub indirect_colors_batch: Allocation,
    pub indices: Allocation,
}

impl PipelinePart for ColorCells {
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
        let cells_to_colorkeys =
            CellsToColorkeys::new(context, cells_to_colorkeys::Settings { workgroup_size });

        let radix_sort = RadixSort::new(
            context,
            radix_sort::Settings {
                workgroup_size,
                dispatch_limit,
                bit_count: 3.try_into().unwrap(),
            },
        );

        let recycle_to_indirect = RecycleToIndirect::new(
            context,
            recycle_to_indirect::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );

        Self {
            cells_to_colorkeys,
            radix_sort,
            recycle_to_indirect,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input { indirect, cell_ids }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let cells_to_colorkeys::Output { keys } = self.cells_to_colorkeys.record(
            context,
            encoder,
            cells_to_colorkeys::Input {
                indirect: indirect.clone(),
                cell_ids,
            },
            cells_to_colorkeys::Parameters,
        )?;
        let radix_sort::Output {
            prefix_sums,
            indices_out,
        } = self.radix_sort.record(
            context,
            encoder,
            radix_sort::Input {
                indirect: indirect.clone(),
                indices_in: None,
                keys,
            },
            radix_sort::Parameters { bit_offset: 0 },
        )?;
        let recycle_to_indirect::Output {
            indirect_colors,
            indirect_colors_batch,
        } = self.recycle_to_indirect.record(
            context,
            encoder,
            recycle_to_indirect::Input {
                indirect,
                count_prefix_sums: prefix_sums,
            },
            recycle_to_indirect::Parameters,
        )?;

        Ok(Output {
            indirect_colors,
            indirect_colors_batch,
            indices: indices_out,
        })
    }
}
