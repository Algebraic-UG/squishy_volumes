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

pub struct BuildCells {
    build_cells: CompiledModule,
    offsets_to_indirect: OffsetsToIndirect,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub cell_size: f32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub positions: Allocation,
    pub prefixed_boundaries: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
            ..
        }: Settings,
        positions: &[Vector4<f32>],
        prefixed_boundaries: &[u32],
    ) -> Self {
        assert_eq!(positions.len(), prefixed_boundaries.len());
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: positions.len() as u32,
        });

        let positions = Allocation::new(device, "positions", positions);
        let indirect = Allocation::new(device, "indirect", &[indirect]);
        let prefixed_boundaries =
            Allocation::new(device, "prefixed_boundaries", prefixed_boundaries);

        Self {
            indirect,
            positions,
            prefixed_boundaries,
        }
    }
}

pub struct Output {
    pub cell_ids: Allocation,
    pub index_ranges: Allocation,
    pub new_indirect: Allocation,
    pub new_indirect_batch: Allocation,
}

impl PipelinePart for BuildCells {
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
        }: Settings,
    ) -> Self {
        let device = context.device();
        let_compiled_module!(
            build_cells,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                    (Vector4::<i32>::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size.get() as f64),
                    ("CELL_SIZE", cell_size as f64),
                ]
            }
        );

        let offsets_to_indirect = OffsetsToIndirect::new(
            context,
            offsets_to_indirect::Settings {
                workgroup_size,
                dispatch_limit,
            },
        );

        Self {
            build_cells,
            offsets_to_indirect,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            positions,
            prefixed_boundaries,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        assert_eq!(
            positions.len::<Vector4<f32>>(),
            prefixed_boundaries.len::<u32>()
        );

        let cell_ids = context
            .allocator()?
            .allocate::<Vector4<i32>>("cell_ids", positions.len::<Vector4<i32>>())?;
        let index_ranges = context
            .allocator()?
            .allocate::<u32>("index_ranges", positions.len::<Vector4<i32>>())?;

        let mut compute_pass = encoder.begin_compute_pass(self.build_cells.label);
        compute_pass.set_pipeline(&self.build_cells.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.build_cells,
                [
                    indirect.binding(),
                    positions.binding(),
                    prefixed_boundaries.binding(),
                    cell_ids.binding(),
                    index_ranges.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());
        drop(compute_pass);

        let offsets_to_indirect::Output {
            new_indirect,
            new_indirect_batch,
        } = self.offsets_to_indirect.record(
            context,
            encoder,
            offsets_to_indirect::Input {
                indirect,
                offsets: prefixed_boundaries,
            },
            offsets_to_indirect::Parameters,
        )?;

        Ok(Output {
            cell_ids,
            index_ranges,
            new_indirect,
            new_indirect_batch,
        })
    }
}
