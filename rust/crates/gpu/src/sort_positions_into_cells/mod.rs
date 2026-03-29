// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

#[cfg(test)]
mod test;

pub struct SortPositionsIntoCells {
    positions_to_keys: PositionsToKeys,
    radix_sort: RadixSort,
}

pub struct SortPositionsIntoCellsSettings {
    pub positions_to_keys_settings: PositionsToKeysSettings,
    pub radix_sort_setttings: RadixSortSettings,
}

pub struct SortPositionsIntoCellsBufferBindings<'a> {
    pub positions: wgpu::BufferBinding<'a>,
    pub radix_sort_buffer_bindings: RadixSortBufferBindings<'a>,
}

impl SortPositionsIntoCells {
    pub fn new(
        context: &GpuContext,
        SortPositionsIntoCellsSettings {
            positions_to_keys_settings,
            radix_sort_setttings,
        }: SortPositionsIntoCellsSettings,
    ) -> Self {
        let positions_to_keys = PositionsToKeys::new(context, positions_to_keys_settings);
        let radix_sort = RadixSort::new(context, radix_sort_setttings);

        Self {
            positions_to_keys,
            radix_sort,
        }
    }

    pub fn compute_in_pass(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        SortPositionsIntoCellsBufferBindings {
            positions,
            radix_sort_buffer_bindings,
        }: &mut SortPositionsIntoCellsBufferBindings,
    ) {
        for dimension in [0, 1, 2] {
            self.positions_to_keys.compute_in_pass(
                context,
                compute_pass,
                positions.clone(),
                radix_sort_buffer_bindings.keys.clone(),
                dimension,
            );
            self.radix_sort
                .compute_in_pass(context, compute_pass, radix_sort_buffer_bindings);
        }
    }
}
