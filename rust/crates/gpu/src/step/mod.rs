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

use nalgebra::{Matrix4x3, Vector4};

use super::*;

pub struct Step {
    sort_positions_into_cells: SortPositionsIntoCells,
    permute_particles: PermuteParticles,
    prepare_grid: PrepareGrid,
    scatter: Scatter,
    collect: Collect,
}

#[derive(Clone)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub bit_count: NonZeroU32,
    pub cell_size: f32,
    pub time_step: f32,
}

pub struct Parameters;

pub struct Input {
    pub indirect_particles: Allocation,
    pub indices_in: Allocation,
    pub masses_in: Allocation,
    pub initial_volumes_in: Allocation,
    pub parameters_in: Allocation,
    pub positions_in: Allocation,
    pub position_gradients_in: Allocation,
    pub velocities_in: Allocation,
    pub velocity_gradients_in: Allocation,
}

#[derive(Clone)]
pub struct InputData<'a> {
    pub indices: &'a [u32],
    pub masses: &'a [f32],
    pub initial_volumes: &'a [f32],
    pub parameters: &'a [particle_parameters::Device],
    pub positions: &'a [Vector4<f32>],
    pub position_gradients: &'a [Matrix4x3<f32>],
    pub velocities: &'a [Vector4<f32>],
    pub velocity_gradients: &'a [Matrix4x3<f32>],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
            ..
        }: Settings,
        InputData {
            indices,
            masses,
            initial_volumes,
            parameters,
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
        }: InputData,
    ) -> Self {
        assert_eq!(indices.len(), masses.len());
        assert_eq!(indices.len(), initial_volumes.len());
        assert_eq!(indices.len(), parameters.len());
        assert_eq!(indices.len(), positions.len());
        assert_eq!(indices.len(), position_gradients.len());
        assert_eq!(indices.len(), velocities.len());
        assert_eq!(indices.len(), velocity_gradients.len());

        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: indices.len() as u32,
        });

        let indirect_particles = Allocation::new(device, "indirect_particles", &[indirect]);
        let indices_in = Allocation::new(device, "indices_in", indices);
        let masses_in = Allocation::new(device, "masses_in", masses);
        let initial_volumes_in = Allocation::new(device, "initial_volumes_in", initial_volumes);
        let parameters_in = Allocation::new(device, "parameters_in", parameters);
        let positions_in = Allocation::new(device, "positions_in", positions);
        let position_gradients_in =
            Allocation::new(device, "position_gradients_in", position_gradients);
        let velocities_in = Allocation::new(device, "velocities_in", velocities);
        let velocity_gradients_in =
            Allocation::new(device, "velocity_gradients_in", velocity_gradients);

        Self {
            indirect_particles,
            indices_in,
            masses_in,
            initial_volumes_in,
            parameters_in,
            positions_in,
            position_gradients_in,
            velocities_in,
            velocity_gradients_in,
        }
    }
}

pub struct Output {
    pub indices_out: Allocation,
    pub masses_out: Allocation,
    pub initial_volumes_out: Allocation,
    pub parameters_out: Allocation,
    pub positions_out: Allocation,
    pub position_gradients_out: Allocation,
    pub velocities_out: Allocation,
    pub velocity_gradients_out: Allocation,
}

pub struct OutputData {
    pub indices_out: Vec<u32>,
    pub masses_out: Vec<f32>,
    pub initial_volumes_out: Vec<f32>,
    pub parameters_out: Vec<particle_parameters::Device>,
    pub positions_out: Vec<Vector4<f32>>,
    pub position_gradients_out: Vec<Matrix4x3<f32>>,
    pub velocities_out: Vec<Vector4<f32>>,
    pub velocity_gradients_out: Vec<Matrix4x3<f32>>,
}

impl PipelinePart for Step {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            dispatch_limit,
            bit_count,
            cell_size,
            time_step,
        }: Settings,
    ) -> Self {
        let sort_positions_into_cells = SortPositionsIntoCells::new(
            context,
            sort_positions_into_cells::Settings {
                workgroup_size,
                dispatch_limit,
                cell_size,
                bit_count,
            },
        );
        let permute_particles =
            PermuteParticles::new(context, permute_particles::Settings { workgroup_size });
        let prepare_grid = PrepareGrid::new(
            context,
            prepare_grid::Settings {
                workgroup_size,
                dispatch_limit,
                cell_size,
            },
        );
        let scatter = Scatter::new(
            context,
            scatter::Settings {
                workgroup_size,
                cell_size,
                time_step,
            },
        );
        let collect = Collect::new(
            context,
            collect::Settings {
                workgroup_size,
                cell_size,
                time_step,
            },
        );

        Self {
            sort_positions_into_cells,
            permute_particles,
            prepare_grid,
            scatter,
            collect,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect_particles,
            indices_in,
            masses_in,
            initial_volumes_in,
            parameters_in,
            positions_in,
            position_gradients_in,
            velocities_in,
            velocity_gradients_in,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let sort_positions_into_cells::Output { permutation } =
            self.sort_positions_into_cells.record(
                context,
                encoder,
                sort_positions_into_cells::Input {
                    indirect: indirect_particles.clone(),
                    positions: positions_in.clone(),
                },
                sort_positions_into_cells::Parameters,
            )?;

        let permute_particles::Output {
            indices_out,
            masses_out,
            initial_volumes_out,
            parameters_out,
            positions_out,
            position_gradients_out,
            velocities_out,
            velocity_gradients_out,
        } = self.permute_particles.record(
            context,
            encoder,
            permute_particles::Input {
                indirect: indirect_particles.clone(),
                permutation,
                indices_in,
                masses_in,
                initial_volumes_in,
                parameters_in,
                positions_in,
                position_gradients_in,
                velocities_in,
                velocity_gradients_in,
            },
            permute_particles::Parameters,
        )?;

        let prepare_grid::Output {
            indirect_cells,
            indirect_cells_batch,
            indirect_colors,
            indirect_colors_batch,
            indirect_blocks,
            cell_indices,
            cell_index_ranges,
            cell_ids,
            block_ids,
            block_table,
        } = self.prepare_grid.record(
            context,
            encoder,
            prepare_grid::Input {
                indirect_particles,
                positions: positions_out.clone(),
            },
            prepare_grid::Parameters,
        )?;
        drop(indirect_cells);
        drop(indirect_colors);
        let scatter::Output { blocks } = self.scatter.record(
            context,
            encoder,
            scatter::Input {
                indirect_colors_batch,
                cell_indices,
                cell_index_ranges: cell_index_ranges.clone(),
                cell_ids: cell_ids.clone(),
                block_ids: block_ids.clone(),
                block_table: block_table.clone(),
                masses: masses_out.clone(),
                initial_volumes: initial_volumes_out.clone(),
                particle_parameters: parameters_out.clone(),
                positions: positions_out.clone(),
                position_gradients: position_gradients_out.clone(),
                velocities: velocities_out.clone(),
                velocity_gradients: velocity_gradients_out.clone(),
            },
            scatter::Parameters,
        )?;

        let collect::Output {
            positions: positions_out,
            position_gradients: position_gradients_out,
            velocities: velocities_out,
            velocity_gradients: velocity_gradients_out,
        } = self.collect.record(
            context,
            encoder,
            collect::Input {
                indirect_cells_batch,
                cell_index_ranges,
                cell_ids,
                block_ids,
                block_table,
                positions: positions_out,
                position_gradients: position_gradients_out,
                velocities: velocities_out,
                velocity_gradients: velocity_gradients_out,
                blocks,
            },
            collect::Parameters,
        )?;

        Ok(Output {
            indices_out,
            masses_out,
            initial_volumes_out,
            parameters_out,
            positions_out,
            position_gradients_out,
            velocities_out,
            velocity_gradients_out,
        })
    }
}
