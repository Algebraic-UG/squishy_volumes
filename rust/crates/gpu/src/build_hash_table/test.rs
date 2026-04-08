// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

fn check(cells: &[Vector4<i32>], indices: &[u32]) {
    let table_size = (cells.len() as u32 * 2).next_power_of_two();
    let mask = table_size - 1;

    for (index, hash) in cells_to_murmur_on_cpu(cells).iter().enumerate() {
        let mut slot = hash & mask;
        loop {
            let index_1 = indices[slot as usize];
            assert!(index_1 > 0);
            if index_1 as usize == index + 1 {
                break;
            }
            slot += 1;
            slot &= mask;
        }
    }
}

#[test]
fn test_simple() {
    let cells = [
        Vector4::new(-5, -5, -5, 0),
        Vector4::new(-5, -5, 5, 0),
        Vector4::new(-5, 5, -5, 0),
        Vector4::new(-5, 5, 5, 0),
        Vector4::new(5, -5, -5, 0),
        Vector4::new(5, -5, 5, 0),
        Vector4::new(5, 5, -5, 0),
        Vector4::new(5, 5, 5, 0),
    ];

    check(&cells, &run_build_hash_table(64, &cells));
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let cells: Vec<i32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<i32>()
        .take(1000 * 4)
        .collect();
    let cells: Vec<Vector4<i32>> = cells
        .chunks_exact(4)
        .map(Vector4::from_column_slice)
        .collect();

    check(&cells, &run_build_hash_table(64, &cells));
}

fn run_build_hash_table(workgroup_size: u32, cells: &[Vector4<i32>]) -> Vec<u32> {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let build_hash_table = BuildHashTable::new(&context, BuildHashTableSettings { workgroup_size });
    let buffers = build_hash_table.create_buffers(&context, BuildHashTableBufferInput { cells });

    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download"),
        size: buffers.indices.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    build_hash_table.compute_in_pass(&context, &mut compute_pass, (&buffers).into(), ());

    drop(compute_pass);
    encoder.copy_buffer_to_buffer(&buffers.indices, 0, &download_buffer, 0, None);

    context.queue().submit([encoder.finish()]);
    let data_buffer_slice = download_buffer.slice(..);
    data_buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let data = data_buffer_slice.get_mapped_range();
    let result: &[u32] = bytemuck::cast_slice(&data);

    result.to_vec()
}
