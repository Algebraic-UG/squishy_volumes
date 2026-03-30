// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector4;

use super::*;

#[test]
fn test_simple() {
    let positions = [
        Vector4::new(-0.5, -0.5, -0.5, 0.),
        Vector4::new(-0.5, -0.5, 0.5, 0.),
        Vector4::new(-0.5, 0.5, -0.5, 0.),
        Vector4::new(-0.5, 0.5, 0.5, 0.),
        Vector4::new(0.5, -0.5, -0.5, 0.),
        Vector4::new(0.5, -0.5, 0.5, 0.),
        Vector4::new(0.5, 0.5, -0.5, 0.),
        Vector4::new(0.5, 0.5, 0.5, 0.),
    ];

    let cell_size = 1.;

    let indices = (0..positions.len() as u32).collect::<Vec<_>>();
    assert_eq!(
        sort_positions_into_cells_on_cpu(&indices, &positions, cell_size),
        run_sort_positions_into_cells(64, cell_size, 2, &indices, &positions),
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let cell_size = 1337.;

    let positions: Vec<f32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<f32>()
        .take(1000 * 4)
        .collect();
    let positions: Vec<Vector4<f32>> = positions
        .chunks_exact(4)
        .map(Vector4::from_column_slice)
        .collect();

    let mut indices: Vec<_> = (0..positions.len() as u32).collect();
    shuffle(&mut indices, 43);

    assert_eq!(
        sort_positions_into_cells_on_cpu(&indices, &positions, cell_size),
        run_sort_positions_into_cells(64, cell_size, 2, &indices, &positions),
    );
}

fn run_sort_positions_into_cells(
    workgroup_size: u32,
    cell_size: f32,
    bit_count: u32,
    indices: &[u32],
    positions: &[Vector4<f32>],
) -> Vec<u32> {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let sort_positions_into_cells = SortPositionsIntoCells::new(
        &context,
        SortPositionsIntoCellsSettings {
            positions_to_keys_settings: PositionsToKeysSettings {
                workgroup_size,
                cell_size,
            },
            radix_sort_setttings: RadixSortSettings {
                count_subkeys_settings: CountSubkeysSettings {
                    workgroup_size,
                    bit_count,
                },
                prefix_sum_settings: PrefixSumSettings { workgroup_size },
                reorder_settings: ReorderSettings {
                    workgroup_size,
                    bit_count,
                },
            },
        },
    );

    let buffers = sort_positions_into_cells.create_buffers(
        &context,
        SortPositionsIntoCellsBufferInput { indices, positions },
    );
    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download"),
        size: buffers.radix_sort.indices_back.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut buffer_bindings = (&buffers).into();

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    sort_positions_into_cells.compute_in_pass(
        &context,
        &mut compute_pass,
        &mut buffer_bindings,
        &mut (),
    );

    drop(compute_pass);
    let last_index_buffer = if buffer_bindings.radix_sort.indices.swapped() {
        buffers.radix_sort.indices_back
    } else {
        buffers.radix_sort.indices_front
    };
    encoder.copy_buffer_to_buffer(&last_index_buffer, 0, &download_buffer, 0, None);

    context.queue().submit([encoder.finish()]);
    let data_buffer_slice = download_buffer.slice(..);
    data_buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let data = data_buffer_slice.get_mapped_range();
    let result: &[u32] = bytemuck::cast_slice(&data);

    result.to_vec()
}
