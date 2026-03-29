// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector4;
use wgpu::util::DeviceExt as _;

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

    let keys_x: [i32; 8] = [-1, -1, -1, -1, 0, 0, 0, 0];
    let keys_y: [i32; 8] = [-1, -1, 0, 0, -1, -1, 0, 0];
    let keys_z: [i32; 8] = [-1, 0, -1, 0, -1, 0, -1, 0];

    assert_eq!(
        positions_to_keys(&positions, 1., 0)
            .into_iter()
            .map(u32_to_i32_offset)
            .collect::<Vec<_>>(),
        keys_x
    );
    assert_eq!(
        positions_to_keys(&positions, 1., 1)
            .into_iter()
            .map(u32_to_i32_offset)
            .collect::<Vec<_>>(),
        keys_y
    );
    assert_eq!(
        positions_to_keys(&positions, 1., 2)
            .into_iter()
            .map(u32_to_i32_offset)
            .collect::<Vec<_>>(),
        keys_z
    );

    let keys_x: Vec<_> = keys_x.into_iter().map(i32_to_u32_offset).collect();
    let keys_y: Vec<_> = keys_y.into_iter().map(i32_to_u32_offset).collect();
    let keys_z: Vec<_> = keys_z.into_iter().map(i32_to_u32_offset).collect();

    assert_eq!(run_positions_to_keys(64, 1., &positions, 0), keys_x);
    assert_eq!(run_positions_to_keys(64, 1., &positions, 1), keys_y);
    assert_eq!(run_positions_to_keys(64, 1., &positions, 2), keys_z);
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

    for dimension in [0, 1, 2] {
        assert_eq!(
            positions_to_keys(&positions, cell_size, dimension),
            run_positions_to_keys(64, cell_size, &positions, dimension),
        );
    }
}

fn run_positions_to_keys(
    workgroup_size: u32,
    cell_size: f32,
    positions: &[Vector4<f32>],
    dimension: u32,
) -> Vec<u32> {
    let context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    let device = context.device();

    let positions_to_keys = PositionsToKeys::new(
        &context,
        PositionsToKeysSettings {
            workgroup_size,
            cell_size,
        },
    );

    let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("positions"),
        contents: bytemuck::cast_slice(positions),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let key_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("keys"),
        size: positions.len() as u64 * u32::MIN_BINDING_SIZE.get(),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download"),
        size: key_buffer.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    positions_to_keys.compute_in_pass(
        &context,
        &mut compute_pass,
        position_buffer.as_entire_buffer_binding(),
        key_buffer.as_entire_buffer_binding(),
        dimension,
    );

    drop(compute_pass);
    encoder.copy_buffer_to_buffer(&key_buffer, 0, &download_buffer, 0, None);

    context.queue().submit([encoder.finish()]);
    let data_buffer_slice = download_buffer.slice(..);
    data_buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let data = data_buffer_slice.get_mapped_range();
    let result: &[u32] = bytemuck::cast_slice(&data);

    result.to_vec()
}
