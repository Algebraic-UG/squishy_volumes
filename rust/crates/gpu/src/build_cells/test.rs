// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

fn check(positions: &[Vector4<f32>], cells: &[Vector4<i32>], index_ranges: &[u32], cell_size: f32) {
    let mut index_start = 0;
    for (index_end, cell) in index_ranges.iter().zip(cells) {
        for index in index_start..index_end + 1 {
            println!("{index}");
            assert_eq!(
                positions[index as usize].map(|c| (c / cell_size).floor() as i32),
                *cell
            );
        }
        index_start = index_end + 1;
    }
}

#[test]
fn test_simple() {
    let workgroup_size = 64;
    let cell_size = 0.3;

    let positions = [
        Vector4::new(0., 0., 0., 0.),
        Vector4::new(0.1, 0., 0., 0.),
        Vector4::new(0.2, 0., 0., 0.),
        Vector4::new(0.3, 0., 0., 0.),
        Vector4::new(0.4, 0., 0., 0.),
        Vector4::new(0.5, 0., 0., 0.),
        Vector4::new(0.6, 0., 0., 0.),
        Vector4::new(0.7, 0., 0., 0.),
    ];

    let prefixed_boundaries = prefix_sum_on_cpu(&[0, 0, 1, 0, 0, 1, 0, 1]);

    let (cells, index_ranges) =
        run_build_cells(workgroup_size, cell_size, &positions, &prefixed_boundaries);

    check(&positions, &cells, &index_ranges, cell_size);
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let workgroup_size = 64;
    let cell_size = 1337.;

    let positions: Vec<f32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<f32>()
        .take(1000 * 4)
        .collect();
    let positions: Vec<Vector4<f32>> = positions
        .chunks_exact(4)
        .map(Vector4::from_column_slice)
        .collect();

    let sorted_indices = sort_positions_into_cells_on_cpu(
        &(0..positions.len() as u32).collect::<Vec<_>>(),
        &positions,
        cell_size,
    );

    let mut positions = positions;
    let lookup = positions.clone();
    sorted_indices
        .iter()
        .zip(&mut positions)
        .for_each(|(index, position)| *position = lookup[*index as usize]);

    let cell_boundaries = find_cell_boundaries_on_cpu(&positions, cell_size);
    let prefixed_boundaries = prefix_sum_on_cpu(&cell_boundaries);

    let (cells, index_ranges) =
        run_build_cells(workgroup_size, cell_size, &positions, &prefixed_boundaries);

    check(&positions, &cells, &index_ranges, cell_size);
}

fn run_build_cells(
    workgroup_size: u32,
    cell_size: f32,
    positions: &[Vector4<f32>],
    prefixed_boundaries: &[u32],
) -> (Vec<Vector4<i32>>, Vec<u32>) {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let build_cells = BuildCells::new(
        &context,
        BuildCellsSettings {
            workgroup_size,
            cell_size,
        },
    );

    let buffers = build_cells.create_buffers(
        &context,
        BuildCellsBufferInput {
            positions,
            prefixed_boundaries,
        },
    );

    let download_cells_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download_cells"),
        size: buffers.cells.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let download_index_ranges_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download_index_ranges"),
        size: buffers.index_ranges.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    build_cells.compute_in_pass(&context, &mut compute_pass, &mut (&buffers).into(), &mut ());

    drop(compute_pass);
    encoder.copy_buffer_to_buffer(&buffers.cells, 0, &download_cells_buffer, 0, None);
    encoder.copy_buffer_to_buffer(
        &buffers.index_ranges,
        0,
        &download_index_ranges_buffer,
        0,
        None,
    );

    context.queue().submit([encoder.finish()]);

    let data_cells_buffer_slice = download_cells_buffer.slice(..);
    data_cells_buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    let data_index_ranges_buffer_slice = download_index_ranges_buffer.slice(..);
    data_index_ranges_buffer_slice.map_async(wgpu::MapMode::Read, |_| {});

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let cells_data = data_cells_buffer_slice.get_mapped_range();
    let cells_result: &[Vector4<i32>] = bytemuck::cast_slice(&cells_data);
    let index_ranges_data = data_index_ranges_buffer_slice.get_mapped_range();
    let index_ranges_result: &[u32] = bytemuck::cast_slice(&index_ranges_data);

    (cells_result.to_vec(), index_ranges_result.to_vec())
}
