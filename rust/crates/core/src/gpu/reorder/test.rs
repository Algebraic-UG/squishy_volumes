// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use wgpu::util::DeviceExt as _;

use super::*;

#[test]
fn test_simple() {
    let bit_count = 5;
    let bit_offset = 0;
    let workgroup_size = 64;
    let subgroup_size = get_subgroup_size();

    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let mut indices: Vec<_> = (0..keys.len() as u32).collect();
    shuffle(&mut indices, 5);

    let counts = count_subkeys_on_cpu(
        bit_count,
        bit_offset,
        workgroup_size,
        subgroup_size,
        &indices,
        &keys,
    );
    let prefixes = prefix_sum_on_cpu(&counts);

    assert_eq!(
        sort_on_cpu_by_bits(bit_count, bit_offset, &indices, &keys),
        run_reorder(
            workgroup_size,
            bit_count,
            bit_offset,
            &indices,
            &keys,
            &prefixes,
        ),
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let bit_count = 2;
    let bit_offset = 0;
    let workgroup_size = 64;
    let subgroup_size = get_subgroup_size();

    let keys: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter()
        .take(1000)
        .collect();
    let mut indices: Vec<_> = (0..keys.len() as u32).collect();
    shuffle(&mut indices, 6);

    let counts = count_subkeys_on_cpu(
        bit_count,
        bit_offset,
        workgroup_size,
        subgroup_size,
        &indices,
        &keys,
    );
    let prefixes = prefix_sum_on_cpu(&counts);

    assert_eq!(
        sort_on_cpu_by_bits(bit_count, bit_offset, &indices, &keys),
        run_reorder(
            workgroup_size,
            bit_count,
            bit_offset,
            &indices,
            &keys,
            &prefixes,
        ),
    );
}

fn run_reorder(
    workgroup_size: u32,
    bit_count: u32,
    bit_offset: u32,
    indices: &[u32],
    keys: &[u32],
    prefixes: &[u32],
) -> Vec<u32> {
    let context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    let device = context.device();

    let reorder = Reorder::new(&context, workgroup_size, bit_count);

    let key_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("keys_in"),
        contents: bytemuck::cast_slice(keys),
        usage: wgpu::BufferUsages::STORAGE,
    });

    assert_eq!(
        prefixes.len() as u32,
        reorder.min_prefixes(keys.len() as u32)
    );
    let prefix_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("prefixes"),
        contents: bytemuck::cast_slice(prefixes),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let index_in_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("index_in"),
        contents: bytemuck::cast_slice(indices),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let index_out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("index_out"),
        size: index_in_buffer.size(),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let download_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download_indices"),
        size: index_out_buffer.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    reorder.compute_in_pass(
        &context,
        &mut compute_pass,
        ReorderBufferBindings {
            keys: key_buffer.as_entire_buffer_binding(),
            prefixes: prefix_buffer.as_entire_buffer_binding(),
            indices_in: index_in_buffer.as_entire_buffer_binding(),
            indices_out: index_out_buffer.as_entire_buffer_binding(),
        },
        bit_offset,
    );

    drop(compute_pass);
    encoder.copy_buffer_to_buffer(&index_out_buffer, 0, &download_index_buffer, 0, None);

    context.queue().submit([encoder.finish()]);

    let data_buffer_index_slice = download_index_buffer.slice(..);
    data_buffer_index_slice.map_async(wgpu::MapMode::Read, |_| {});

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let data_indices = data_buffer_index_slice.get_mapped_range();
    let indices: &[u32] = bytemuck::cast_slice(&data_indices);

    indices.to_vec()
}
