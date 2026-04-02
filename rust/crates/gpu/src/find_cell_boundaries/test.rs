// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

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

    assert_eq!(
        vec![0, 0, 1, 0, 0, 1, 0, 1],
        run_find_cell_boundaries(workgroup_size, cell_size, &positions)
    );
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

    let boundaries = run_find_cell_boundaries(workgroup_size, cell_size, &positions);

    for (index, boundary) in boundaries.into_iter().enumerate() {
        if index + 1 == positions.len() {
            assert_eq!(1, boundary);
            continue;
        }

        let should_be_boundary = positions[index].map(|c| (c / cell_size).floor() as i32)
            != positions[index + 1].map(|c| (c / cell_size).floor() as i32);

        match boundary {
            0 => assert!(!should_be_boundary),
            1 => assert!(should_be_boundary),
            _ => panic!(),
        }
    }
}

fn run_find_cell_boundaries(
    workgroup_size: u32,
    cell_size: f32,
    positions: &[Vector4<f32>],
) -> Vec<u32> {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let find_cell_boundaries = FindCellBoundaries::new(
        &context,
        FindCellBoundariesSettings {
            workgroup_size,
            cell_size,
        },
    );

    let buffers =
        find_cell_boundaries.create_buffers(&context, FindCellBoundariesBufferInput { positions });

    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download"),
        size: buffers.boundaries.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    find_cell_boundaries.compute_in_pass(
        &context,
        &mut compute_pass,
        &mut (&buffers).into(),
        &mut (),
    );

    drop(compute_pass);
    encoder.copy_buffer_to_buffer(&buffers.boundaries, 0, &download_buffer, 0, None);

    context.queue().submit([encoder.finish()]);
    let data_buffer_slice = download_buffer.slice(..);
    data_buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let data = data_buffer_slice.get_mapped_range();
    let result: &[u32] = bytemuck::cast_slice(&data);

    result.to_vec()
}
